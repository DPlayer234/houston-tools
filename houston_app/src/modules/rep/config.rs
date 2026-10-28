use jiff::SignedDuration;
use serde_with::As;

use crate::helper::time::DhmsDuration;

fn default_cooldown() -> SignedDuration {
    const { SignedDuration::from_hours(20) }
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(with = "As::<DhmsDuration>", default = "default_cooldown")]
    pub cooldown: SignedDuration,
    #[serde(default, alias = "cash")]
    pub cash_gain: u32,
}
