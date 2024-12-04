use log4rs::config::Deserializers;

mod default_appender;
mod default_pattern;
mod webhook_appender;

pub fn deserializers() -> Deserializers {
    let mut d = Deserializers::new();
    d.insert("default", default_appender::DefaultAppenderDeserializer);
    d.insert("default", default_pattern::DefaultPatternDeserializer);
    d.insert("webhook", webhook_appender::WebhookAppenderDeserializer);
    d
}