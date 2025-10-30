use chrono::TimeDelta;
use houston_utils::time::TimeDeltaStr;
use serde_with::As;

fn default_cooldown() -> TimeDelta {
    const { TimeDelta::hours(20) }
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(with = "As::<TimeDeltaStr>", default = "default_cooldown")]
    pub cooldown: TimeDelta,
    #[serde(default, alias = "cash")]
    pub cash_gain: u32,
}
