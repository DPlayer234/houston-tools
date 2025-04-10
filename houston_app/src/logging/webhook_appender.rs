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

use std::io::{self, Write as _};
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

use super::WRITE_BUF_SIZE;
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
    sender: Sender<Msg>,
    encoder: Box<dyn Encode>,
}

impl Append for WebhookAppender {
    fn append(&self, record: &Record<'_>) -> Result {
        let mut buf = LogWriter::default();
        self.encoder.encode(&mut buf, record)?;
        self.sender.try_send(Msg::Log(buf.buf.to_vec()))?;
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

// Discord messages are limited to 2000 characters.
// we use 1984 instead of 2000 to also consider a little bit of extra space we
// use for formatting. when a message would exceed this limit, it's re-queued
// instead of being added to the batch.
const BATCH_TEXT_LIMIT: usize = 1984;

/// A temporary buffer for messages to write to the webhook.
///
/// Message data in excess of the capacity is discarded.
#[derive(Debug, Default)]
struct LogWriter {
    /// The buffer of data to write to the webhook.
    buf: ArrayVec<u8, WRITE_BUF_SIZE>,
}

#[derive(Debug)]
enum Msg {
    Log(Vec<u8>),
    Notify(OwnedSemaphorePermit),
}

impl io::Write for LogWriter {
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

impl encode::Write for LogWriter {
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

/// Helper for batching receives.
#[derive(Debug)]
struct BatchReceiver {
    receiver: Receiver<Msg>,
    next_data: Option<Vec<u8>>,
    config: InnerConfig,
    count: usize,
}

impl BatchReceiver {
    /// Take the first item and reset the count.
    ///
    /// Returns [`None`] when the receiver is closed and exhausted.
    async fn first(&mut self) -> Option<Msg> {
        self.count = 1;
        match self.next_data.take() {
            Some(data) => Some(Msg::Log(data)),
            None => self.receiver.recv().await,
        }
    }

    /// Takes another item for the batch, with respect to the limits.
    ///
    /// Does not increase the count itself.
    async fn take_batch(&mut self) -> Option<Msg> {
        if self.count >= self.config.batch_size {
            return None;
        }

        try_recv_timeout(&mut self.receiver, self.config.batch_time).await
    }
}

async fn worker(webhook: WebhookClient, receiver: Receiver<Msg>, config: InnerConfig) {
    // we will reuse this buffer for _every_ posted message
    let mut text = String::new();

    // holds the semaphore permits to notify after the current batch.
    // notified by clearing the vec and thus dropping the permits. buffer is reused.
    #[expect(clippy::collection_is_never_read, reason = "used to delay drop")]
    let mut flush = Vec::new();

    let mut batch = BatchReceiver {
        receiver,
        config,
        next_data: None,
        count: 0,
    };

    // this loop will exit when the sender is dropped and exhausted
    while let Some(data) = batch.first().await {
        // the permit in `Msg::Notify` is implicitly dropped
        let Msg::Log(buf) = data else {
            continue;
        };

        // make sure there's enough space for at least the first message in the batch
        text.reserve(buf.len() + 16);
        text.push_str("```ansi\n");

        push_str_lossy(&mut text, &buf);

        // push additional messages into the buffer if allowed & possible
        while let Some(data) = batch.take_batch().await {
            match data {
                // write the additional log message
                Msg::Log(buf) => {
                    // if there is not enough space left, handle it in the next iteration
                    if text.len() + buf.len() > BATCH_TEXT_LIMIT {
                        batch.next_data = Some(buf);
                        break;
                    }

                    // push the text and increase the batch count
                    push_str_lossy(&mut text, &buf);
                    batch.count += 1;
                },
                // take out the permit to drop it later
                Msg::Notify(notify) => flush.push(notify),
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
                batch.config.wait,
                Vec::new(),
                &ExecuteWebhook::new().content(&text),
            )
            .await;

        // clear the buffer for the next batch
        text.clear();

        if let Err(why) = res {
            eprintln!("webhook appender failed: {why:?}");
        }

        // drop all obtained permits to notify queuers
        flush.clear();
    }
}

/// Equivalent to `receiver.recv()` but using a timeout.
async fn try_recv_timeout(receiver: &mut Receiver<Msg>, timeout: Duration) -> Option<Msg> {
    let task = receiver.recv();
    tokio::time::timeout(timeout, task)
        .await
        .unwrap_or_default()
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

fn try_flush(sender: &Sender<Msg>) {
    // we're not _too_ concerned about the efficiency of this code
    // it really only runs on exit, so the only concern is really
    // "doesn't block other threads"

    // async-over-sync from an async context is... great
    let res: Result = tokio::task::block_in_place(move || {
        tokio::runtime::Handle::current().block_on(async {
            let semaphore = Arc::new(Semaphore::new(1));
            let notify = Arc::clone(&semaphore).acquire_owned().await?;
            let msg = Msg::Notify(notify);

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
