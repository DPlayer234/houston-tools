//! An appender that logs to a Discord webhook.
//!
//! The actual appender pushes buffers to a sender, which is then handled by a
//! worker task. The size of individual _formatted_ messages is limited
//! to avoid having to allocate additional memory on every logging call.
//!
//! The worker task tries to batch messages that arrive close in time to reduce
//! the amount of calls to the webhook, but this is limited by the maximum
//! message size. This probably shouldn't be used for all logging messages but
//! just a subset, i.e. filtered to just warnings and errors.

use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

use arrayvec::ArrayVec;
use log::Record;
use log4rs::append::Append;
use log4rs::config::{Deserialize, Deserializers};
use log4rs::encode::{self, Encode, EncoderConfig, Style};
use serenity::http::Http;
use serenity::secrets::SecretString;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::prelude::*;

// set a time limit for flushing so we don't block the app unnecessarily.
// generally, flushing only happens on exit, so blocking is fine, however,
// since this may prevent a shutdown-and-restart, this time may inhibit a
// restart. furthermore, because the expected use for this appender are
// _warnings and errors only_, chances are if we exceed this time budget,
// things are already really bad
const FLUSH_TIME_LIMIT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct WebhookAppender {
    sender: Sender<LogData>,
    encoder: Box<dyn Encode>,
}

impl Append for WebhookAppender {
    fn append(&self, record: &Record<'_>) -> Result {
        let mut buf = LogData::default();
        self.encoder.encode(&mut buf, record)?;
        self.sender.try_send(buf)?;
        Ok(())
    }

    fn flush(&self) {
        try_flush(&self.sender);
    }
}

#[derive(Debug)]
struct WebhookClient {
    http: Http,
    id: WebhookId,
    token: SecretString,
}

impl WebhookClient {
    fn new(url: &str) -> Result<Self> {
        let url = url::Url::parse(url)?;
        let (id, token) =
            serenity::utils::parse_webhook(&url).context("cannot parse webhook url")?;

        let http = Http::without_token();
        let token = SecretString::new(Arc::from(token));

        Ok(Self { http, id, token })
    }
}

const LOG_DATA_SIZE: usize = 256;

/// Represents a single log message to be written to the webhook.
#[derive(Debug, Default)]
struct LogData {
    /// The buffer of data to write to the webhook.
    buf: ArrayVec<u8, LOG_DATA_SIZE>,
    /// A semaphore permit to release after the message is written.
    notify: Option<OwnedSemaphorePermit>,
}

impl io::Write for LogData {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_all(buf)?;
        Ok(buf.len())
    }

    fn write_all(&mut self, mut buf: &[u8]) -> io::Result<()> {
        let rem = self.buf.remaining_capacity();
        if rem == 0 {
            return Ok(());
        }

        if rem < buf.len() {
            buf = &buf[..rem];
        }

        let res = self.buf.try_extend_from_slice(buf);
        debug_assert!(res.is_ok(), "must be okay since we trimmed it");
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl encode::Write for LogData {
    fn set_style(&mut self, style: &Style) -> io::Result<()> {
        use log4rs::encode::Color;

        self.write_all(b"\x1b[0m")?;

        if let Some(text) = style.text {
            self.write_all(if style.intense == Some(true) {
                b"\x1b[1;"
            } else {
                b"\x1b[0;"
            })?;

            match text {
                Color::Black => self.write_all(b"30m"),
                Color::Red => self.write_all(b"31m"),
                Color::Green => self.write_all(b"32m"),
                Color::Yellow => self.write_all(b"33m"),
                Color::Blue => self.write_all(b"34m"),
                Color::Magenta => self.write_all(b"35m"),
                Color::Cyan => self.write_all(b"36m"),
                Color::White => self.write_all(b"37m"),
            }?;
        }

        Ok(())
    }
}

async fn worker(webhook: WebhookClient, mut receiver: Receiver<LogData>, config: InnerConfig) {
    // Discord messages are limited to 2000 characters.
    // to avoid hitting this limit, don't batch more if we can't guarantee
    // that the next log message can still fit. we use 1984 instead of 2000
    // to also consider a little bit of extra space we use for formatting.
    const SIZE_CAP: usize = 1984 - LOG_DATA_SIZE;

    // we will reuse this buffer for _every_ posted message
    let mut text = String::new();

    // holds the semaphore permits to notify after the current batch.
    // notified by clearing the vec and thus dropping the permits. buffer is reused.
    #[allow(clippy::collection_is_never_read, reason = "used to delay drop")]
    let mut flush = Vec::new();

    // this loop will exit when the sender is dropped
    while let Some(data) = receiver.recv().await {
        // the permit of `data` is implicitly dropped
        if data.buf.is_empty() {
            continue;
        }

        // make sure there's enough space for at least the first message in the batch
        text.reserve(data.buf.len() + 16);
        text.push_str("```ansi\n");

        push_str_lossy(&mut text, &data.buf);

        // push additional messages into the buffer if allowed & possible
        if config.batch_size > 1 {
            let mut count = 1usize;
            while let Some(mut data) = try_recv_timeout(&mut receiver, config.batch_time).await {
                // take out the permit to drop it later
                if let Some(sem) = data.notify.take() {
                    flush.push(sem);
                }

                count += 1;
                push_str_lossy(&mut text, &data.buf);

                if text.len() > SIZE_CAP || count >= config.batch_size {
                    break;
                }
            }
        }

        // include a new-line here also
        // if it didn't, this will ensure the formatting is correct
        // if the last message ended with a new-line, it won't render twice
        text.push_str("\n```");

        let res = webhook
            .http
            .execute_webhook(
                webhook.id,
                None,
                webhook.token.expose_secret(),
                config.wait,
                Vec::new(),
                &ExecuteWebhook::new().content(&text),
            )
            .await;

        // clear the buffer for the next batch
        text.clear();

        if let Err(why) = res {
            eprintln!("webhook appender failed: {why:?}");
        }

        flush.clear();
    }
}

/// Equivalent to `receiver.recv()` but using a timeout.
async fn try_recv_timeout(receiver: &mut Receiver<LogData>, timeout: Duration) -> Option<LogData> {
    let task = receiver.recv();
    tokio::time::timeout(timeout, task).await.ok().flatten()
}

/// Lossy-decodes `buf` and appends it to `target` as one operation.
///
/// Invalid sequences are replaced by [`char::REPLACEMENT_CHARACTER`].
fn push_str_lossy(target: &mut String, buf: &[u8]) {
    for chunk in buf.utf8_chunks() {
        target.push_str(chunk.valid());

        if !chunk.invalid().is_empty() {
            target.push(char::REPLACEMENT_CHARACTER);
        }
    }
}

fn try_flush(sender: &Sender<LogData>) {
    // we're not _too_ concerned about the efficiency of this code
    // it really only runs on exit, so the only concern is really
    // "doesn't block other threads"

    // async-over-sync from an async context is... great
    let res: Result = tokio::task::block_in_place(move || {
        tokio::runtime::Handle::current().block_on(async {
            let semaphore = Arc::new(Semaphore::new(1));
            let notify = Arc::clone(&semaphore).acquire_owned().await?;

            let msg = LogData {
                buf: ArrayVec::new(),
                notify: Some(notify),
            };

            // we essentially wait until the semaphore lets us grab another permit
            // this should happen when `notify` of the `msg` is dropped,
            // which happens after the final batch send
            let task = async move {
                sender.send(msg).await?;
                _ = semaphore.acquire().await?;
                Ok(())
            };

            // use the time limit here to not block forever
            tokio::time::timeout(FLUSH_TIME_LIMIT, task).await?
        })
    });

    if let Err(why) = res {
        eprintln!("could not flush webhook appender: {why:?}");
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct WebhookAppenderConfig {
    url: SecretString,
    encoder: EncoderConfig,
    #[serde(flatten)]
    inner: InnerConfig,
}

fn default_buffer_size() -> usize {
    32
}

fn default_batch_size() -> usize {
    10
}

fn default_batch_time() -> Duration {
    Duration::from_millis(500)
}

#[derive(Debug, serde::Deserialize)]
struct InnerConfig {
    #[serde(default = "default_buffer_size")]
    buffer_size: usize,
    #[serde(default = "default_batch_size")]
    batch_size: usize,
    #[serde(default = "default_batch_time")]
    batch_time: Duration,
    #[serde(default)]
    wait: bool,
}

pub struct WebhookAppenderDeserializer;

impl Deserialize for WebhookAppenderDeserializer {
    type Trait = dyn Append;
    type Config = WebhookAppenderConfig;

    fn deserialize(
        &self,
        config: Self::Config,
        deserializers: &Deserializers,
    ) -> Result<Box<Self::Trait>> {
        let encoder = deserializers.deserialize(&config.encoder.kind, config.encoder.config)?;

        let client = WebhookClient::new(config.url.expose_secret())?;
        let (sender, receiver) = channel(config.inner.buffer_size);

        let appender = WebhookAppender { sender, encoder };

        tokio::spawn(worker(client, receiver, config.inner));
        Ok(Box::new(appender))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_str_lossy() {
        // "Hello " + invalid UTF-8 + "World!"
        let buf = b"Hello \xF0\x90\x80World!";
        let mut target = String::new();
        push_str_lossy(&mut target, buf);
        assert_eq!(target, "Hello �World!");
    }

    #[test]
    fn test_push_str_lossy_only_valid() {
        let buf = b"Hello World!";
        let mut target = String::new();
        push_str_lossy(&mut target, buf);
        assert_eq!(target, "Hello World!");
    }

    #[test]
    fn test_push_str_lossy_only_invalid() {
        // invalid UTF-8
        let buf = b"\x80\x80\x80\x80";
        let mut target = String::new();
        push_str_lossy(&mut target, buf);
        assert_eq!(target, "����");
    }

    #[test]
    fn test_push_str_lossy_mixed() {
        // "Valid " + invalid UTF-8 + "Invalid " + valid UTF-8 (snowman)
        let buf = b"Valid \xF0\x90\x80Invalid \xE2\x98\x83";
        let mut target = String::new();
        push_str_lossy(&mut target, buf);
        assert_eq!(target, "Valid �Invalid ☃");
    }
}
