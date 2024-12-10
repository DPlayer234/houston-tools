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
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::prelude::*;

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

    fn flush(&self) {}
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

const LOG_DATA_SIZE: usize = 252;

#[derive(Debug, Default)]
struct LogData {
    buf: ArrayVec<u8, LOG_DATA_SIZE>,
}

const _: () = assert!(
    size_of::<LogData>() <= 256,
    "LogData should be at most 256 bytes in size",
);

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

    // this loop will exit when the sender is dropped
    while let Some(data) = receiver.recv().await {
        // make sure there's enough space for at least the first message in the batch
        text.reserve(data.buf.len() + 16);
        text.push_str("```ansi\n");

        push_str_lossy(&mut text, &data.buf);

        // push additional messages into the buffer if allowed & possible
        if config.batch_size > 1 {
            let mut count = 0usize;
            while let Some(data) = try_recv_timeout(&mut receiver, config.batch_time).await {
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
    }
}

/// Equivalent to `receiver.recv()` but using a timeout.
async fn try_recv_timeout(receiver: &mut Receiver<LogData>, timeout: Duration) -> Option<LogData> {
    tokio::time::timeout(timeout, receiver.recv())
        .await
        .ok()
        .flatten()
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
