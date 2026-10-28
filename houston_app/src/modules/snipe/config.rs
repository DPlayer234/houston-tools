use std::collections::HashMap;
use std::num::NonZero;
use std::sync::Mutex;

use jiff::SignedDuration;
use serde_with::As;

use super::state::GuildState;
use crate::helper::time::DhmsDuration;
use crate::prelude::*;

fn default_max_age() -> SignedDuration {
    const { SignedDuration::from_mins(5) }
}

fn default_max_cache_size() -> NonZero<usize> {
    const { NonZero::new(64).unwrap() }
}

pub type Config = HashMap<GuildId, GuildConfig>;

#[derive(Debug, serde::Deserialize)]
pub struct GuildConfig {
    #[serde(with = "As::<DhmsDuration>", default = "default_max_age")]
    pub max_age: SignedDuration,
    #[serde(default = "default_max_cache_size")]
    pub max_cache_size: NonZero<usize>,

    #[serde(skip)]
    pub state: Mutex<GuildState>,
}
