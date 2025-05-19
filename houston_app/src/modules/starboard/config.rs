use std::collections::HashMap;

use extract_map::ExtractKey;
use serenity::small_fixed_array::{FixedArray, FixedString};
use tokio::sync::Semaphore;

use crate::config::HEmoji;
use crate::helper::index_extract_map::IndexExtractMap;
use crate::prelude::*;

pub type Config = HashMap<GuildId, StarboardGuild>;

#[derive(
    Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct BoardId(i64);

impl BoardId {
    pub const fn new(id: i64) -> Self {
        Self(id)
    }

    pub const fn get(self) -> i64 {
        self.0
    }
}

impl ExtractKey<BoardId> for StarboardEntry {
    fn extract_key(&self) -> &BoardId {
        &self.id
    }
}

impl From<i64> for BoardId {
    fn from(value: i64) -> Self {
        Self::new(value)
    }
}

fn pin_lock() -> Semaphore {
    Semaphore::new(1)
}

#[derive(Debug, serde::Deserialize)]
pub struct StarboardGuild {
    #[serde(default)]
    pub remove_score_on_delete: bool,
    pub boards: IndexExtractMap<BoardId, StarboardEntry>,

    #[serde(skip, default = "pin_lock")]
    pub pin_lock: Semaphore,
}

#[derive(Debug, serde::Deserialize)]
pub struct StarboardEntry {
    pub id: BoardId,
    pub name: FixedString<u8>,
    pub channel: GenericChannelId,
    pub emoji: HEmoji,
    pub reacts: u32,
    #[serde(default = "FixedArray::new")]
    pub notices: FixedArray<FixedString>,
    #[serde(default, alias = "cash")]
    pub cash_gain: i32,
    #[serde(default, alias = "cash_pin")]
    pub cash_pin_gain: i32,
}

impl StarboardEntry {
    pub fn any_cash_gain(&self) -> bool {
        self.cash_gain != 0 || self.cash_pin_gain != 0
    }
}
