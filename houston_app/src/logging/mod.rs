use log4rs::config::Deserializers;

mod default_appender;
mod default_pattern;
mod target_filter;
mod webhook_appender;

// stack buffer size used for custom appenders.
// 1 KiB should be sufficient for most messages.
const WRITE_BUF_SIZE: usize = 0x400;

pub fn deserializers() -> Deserializers {
    let mut d = Deserializers::new();
    d.insert("default", default_appender::DefaultAppenderDeserializer);
    d.insert("default", default_pattern::DefaultPatternDeserializer);
    d.insert("target", target_filter::TargetFilterDeserializer);
    d.insert("webhook", webhook_appender::WebhookAppenderDeserializer);
    d
}
