use std::collections::HashMap;
use std::num::NonZero;
use std::sync::Mutex;

use chrono::TimeDelta;
use houston_utils::time::TimeDeltaStr;
use serde_with::As;

use super::state::GuildState;
use crate::prelude::*;

fn default_max_age() -> TimeDelta {
    const { TimeDelta::minutes(5) }
}

fn default_max_cache_size() -> NonZero<usize> {
    const { NonZero::new(64).unwrap() }
}

pub type Config = HashMap<GuildId, GuildConfig>;

#[derive(Debug, serde::Deserialize)]
pub struct GuildConfig {
    #[serde(with = "As::<TimeDeltaStr>", default = "default_max_age")]
    pub max_age: TimeDelta,
    #[serde(default = "default_max_cache_size")]
    pub max_cache_size: NonZero<usize>,

    #[serde(skip)]
    pub state: Mutex<GuildState>,
}
