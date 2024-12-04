use std::sync::Arc;

use log::{Level, Record};
use log4rs::append::Append;
use log4rs::config::{Deserialize, Deserializers};
use serenity::http::Http;
use serenity::secrets::SecretString;

use utils::text::truncate;

use crate::fmt::written_or;
use crate::prelude::*;

#[derive(Debug)]
pub struct WebhookAppender {
    inner: Arc<WebhookAppenderInner>,
}

#[derive(Debug)]
struct WebhookAppenderInner {
    http: Http,
    id: WebhookId,
    token: SecretString,
}

impl WebhookAppender {
    fn new(url: &str) -> Result<Self> {
        let url = url::Url::parse(url)?;
        let (id, token) = serenity::utils::parse_webhook(&url).context("cannot parse webhook url")?;

        let http = Http::without_token();
        let token = SecretString::new(Arc::from(token));

        Ok(Self {
            inner: Arc::new(WebhookAppenderInner {
                http,
                id,
                token,
            }),
        })
    }
}

impl Append for WebhookAppender {
    fn append(&self, record: &Record<'_>) -> Result {
        let display = LevelInfo::for_level(record.level());
        let target = truncate(record.target(), 250).into_owned();
        let message = truncate(record.args().to_string(), 4000);

        let embed = CreateEmbed::new()
            .title(display.label)
            .color(display.color)
            .author(CreateEmbedAuthor::new(target))
            .description(written_or(message, "<empty log message>"))
            .timestamp(Timestamp::now());

        let this = Arc::clone(&self.inner);
        tokio::spawn(async move {
            let res = this.http.execute_webhook(
                this.id,
                None,
                this.token.expose_secret(),
                false,
                Vec::new(),
                &ExecuteWebhook::new().embed(embed),
            ).await;

            if let Err(why) = res {
                eprintln!("webhook logger error: {why:?}");
            }
        });

        Ok(())
    }

    fn flush(&self) {}
}

struct LevelInfo {
    label: &'static str,
    color: Color,
}

impl LevelInfo {
    const fn for_level(level: Level) -> Self {
        match level {
            Level::Error => Self {
                label: "🚨 Error",
                color: Colour(0xEA3333),
            },
            Level::Warn => Self {
                label: "⚠️ Warning",
                color: Colour(0xEFDD10),
            },
            Level::Info => Self {
                label: "ℹ️ Info",
                color: Colour(0x0DB265),
            },
            Level::Debug => Self {
                label: "🧩 Debug",
                color: Colour(0xCCCCCC),
            },
            Level::Trace => Self {
                label: "Trace",
                color: Colour(0x119FB7),
            },
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct WebhookAppenderConfig {
    url: SecretString,
}

pub struct WebhookAppenderDeserializer;

impl Deserialize for WebhookAppenderDeserializer {
    type Trait = dyn Append;
    type Config = WebhookAppenderConfig;

    fn deserialize(
        &self,
        config: Self::Config,
        _deserializers: &Deserializers,
    ) -> Result<Box<Self::Trait>> {
        let appender = WebhookAppender::new(config.url.expose_secret())?;
        Ok(Box::new(appender))
    }
}
