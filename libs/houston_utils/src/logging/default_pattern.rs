//! Defines a `"default"` [`PatternEncoder`].
//!
//! This just serves to avoid repeating the pattern multiple times in the
//! configuration.

use log4rs::config::Deserialize;
use log4rs::encode::Encode;
use log4rs::encode::pattern::PatternEncoder;

fn default_time() -> bool {
    true
}

#[derive(Debug, serde::Deserialize)]
pub struct DefaultPatternConfig {
    #[serde(default = "default_time")]
    time: bool,
}

pub struct DefaultPatternDeserializer;

impl Deserialize for DefaultPatternDeserializer {
    type Trait = dyn Encode;
    type Config = DefaultPatternConfig;

    fn deserialize(
        &self,
        config: Self::Config,
        _deserializers: &log4rs::config::Deserializers,
    ) -> anyhow::Result<Box<Self::Trait>> {
        let pattern = if config.time {
            "[{d(%Y-%m-%d %H:%M:%S)(utc)} {h({l:<5})} {t}] {m}{n}"
        } else {
            "[{h({l:<5})} {t}] {m}{n}"
        };

        Ok(Box::new(PatternEncoder::new(pattern)))
    }
}
