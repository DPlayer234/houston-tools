use serde_with::As;
use time::Duration;

use crate::helper::time::DhmsDuration;

fn default_cooldown() -> Duration {
    const { Duration::hours(20) }
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(with = "As::<DhmsDuration>", default = "default_cooldown")]
    pub cooldown: Duration,
    #[serde(default, alias = "cash")]
    pub cash_gain: u32,
}
