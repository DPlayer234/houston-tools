use log::Record;
use log4rs::config::Deserialize;
use log4rs::filter::{Filter, Response};

/// A filter that allows filtering an appender by targets.
///
/// Also allows forcefully accepting targets, overriding later filters.
/// This is useful to always log certain targets even when an appender is
/// filtered via a level [`threshold`].
///
/// [`threshold`]: log4rs::filter::threshold
#[derive(Debug)]
pub struct TargetFilter {
    config: TargetFilterConfig,
}

impl Filter for TargetFilter {
    fn filter(&self, record: &Record<'_>) -> Response {
        let is_match = record.target().starts_with(&self.config.target);
        match (self.config.mode, is_match) {
            (TargetMode::RejectMismatch, false) => Response::Reject,
            (TargetMode::AcceptMismatch, false) => Response::Accept,
            (TargetMode::RejectMatch, true) => Response::Reject,
            (TargetMode::AcceptMatch, true) => Response::Accept,
            _ => Response::Neutral,
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum TargetMode {
    RejectMismatch,
    AcceptMismatch,
    RejectMatch,
    AcceptMatch,
}

#[derive(Debug, serde::Deserialize)]
pub struct TargetFilterConfig {
    target: String,
    mode: TargetMode,
}

pub struct TargetFilterDeserializer;

impl Deserialize for TargetFilterDeserializer {
    type Trait = dyn Filter;
    type Config = TargetFilterConfig;

    fn deserialize(
        &self,
        config: Self::Config,
        _deserializers: &log4rs::config::Deserializers,
    ) -> anyhow::Result<Box<Self::Trait>> {
        Ok(Box::new(TargetFilter { config }))
    }
}
