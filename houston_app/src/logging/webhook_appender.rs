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

        // just swallow messages if we hit the limit
        _ = self.sender.try_send(buf);
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
        let (id, token) = serenity::utils::parse_webhook(&url).context("cannot parse webhook url")?;

        let http = Http::without_token();
        let token = SecretString::new(Arc::from(token));

        Ok(Self {
            http,
            id,
            token,
        })
    }
}

#[derive(Debug, Default)]
struct LogData {
    buf: ArrayVec<u8, 252>,
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

async fn worker(webhook: WebhookClient, mut receiver: Receiver<LogData>) {
    while let Some(data) = receiver.recv().await {
        let mut text = String::with_capacity(data.buf.len() + 16);
        text.push_str("```ansi\n");

        push_str_lossy(&mut text, &data.buf);

        if text.len() < 1500 {
            let mut count = 0usize;
            'batch: while let Some(data) = try_recv_timeout(&mut receiver).await {
                count += 1;
                push_str_lossy(&mut text, &data.buf);

                if text.len() >= 1500 || count >= 10 {
                    break 'batch;
                }
            }
        }

        text.push_str("```");

        let res = webhook.http.execute_webhook(
            webhook.id,
            None,
            webhook.token.expose_secret(),
            false,
            Vec::new(),
            &ExecuteWebhook::new().content(text),
        ).await;

        if let Err(why) = res {
            eprintln!("webhook appender failed: {why:?}");
        }
    }
}

async fn try_recv_timeout<T>(receiver: &mut Receiver<T>) -> Option<T> {
    for _ in 0..10 {
        if let Ok(data) = receiver.try_recv() {
            return Some(data);
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    None
}

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
        let client = WebhookClient::new(config.url.expose_secret())?;

        let (sender, receiver) = channel(32);
        let encoder = deserializers.deserialize(&config.encoder.kind, config.encoder.config)?;

        let appender = WebhookAppender {
            sender,
            encoder,
        };

        tokio::spawn(worker(client, receiver));
        Ok(Box::new(appender))
    }
}
