use chrono::TimeDelta;

use crate::helper::time::serde_time_delta;

fn default_cooldown() -> TimeDelta {
    const { TimeDelta::hours(20) }
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(with = "serde_time_delta", default = "default_cooldown")]
    pub cooldown: TimeDelta,
    pub cash: u32,
}
