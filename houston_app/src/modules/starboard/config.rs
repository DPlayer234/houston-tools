use std::collections::HashMap;
use std::fmt;

use indexmap::IndexMap;
use tokio::sync::Semaphore;

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
    #[serde(with = "board_order_fix")]
    pub boards: IndexMap<BoardId, StarboardEntry>,

    #[serde(skip, default = "pin_lock")]
    pub pin_lock: Semaphore,
}

mod board_order_fix {
    use serde::de::{Deserialize as _, Deserializer};

    use super::*;

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<IndexMap<BoardId, StarboardEntry>, D::Error> {
        let mut map = <IndexMap<BoardId, StarboardEntry>>::deserialize(deserializer)?;
        map.sort_unstable_by(|k1, v1, k2, v2| v1.sort.cmp(&v2.sort).reverse().then(k1.cmp(k2)));
        Ok(map)
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct StarboardEntry {
    pub name: String,
    pub channel: ChannelId,
    pub emoji: StarboardEmoji,
    pub reacts: u32,
    #[serde(default = "Vec::new")]
    pub notices: Vec<String>,
    #[serde(default)]
    pub cash_gain: i32,
    #[serde(default)]
    pub cash_pin_gain: i32,
    #[serde(default)]
    pub sort: i8,
}

impl StarboardEntry {
    pub fn any_cash_gain(&self) -> bool {
        self.cash_gain != 0 || self.cash_pin_gain != 0
    }
}

#[derive(Debug)]
pub struct StarboardEmoji(ReactionType);

impl StarboardEmoji {
    pub fn as_emoji(&self) -> &ReactionType {
        &self.0
    }

    pub fn equivalent_to(&self, reaction: &ReactionType) -> bool {
        match (self.as_emoji(), reaction) {
            (
                ReactionType::Custom { id: self_id, .. },
                ReactionType::Custom { id: other_id, .. },
            ) => self_id == other_id,
            (ReactionType::Unicode(self_name), ReactionType::Unicode(other_name)) => {
                self_name == other_name
            },
            _ => false,
        }
    }
}

impl fmt::Display for StarboardEmoji {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de> serde::Deserialize<'de> for StarboardEmoji {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::fmt;

        use serenity::small_fixed_array::FixedString;

        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = StarboardEmoji;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("string for emoji")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let emoji = if let Some((name, id)) = v.split_once(':') {
                    let id = id.parse().map_err(|_| E::custom("invalid emoji id"))?;
                    let name = Some(FixedString::from_str_trunc(name));
                    ReactionType::Custom {
                        animated: false,
                        id,
                        name,
                    }
                } else {
                    ReactionType::Unicode(FixedString::from_str_trunc(v))
                };

                Ok(StarboardEmoji(emoji))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}
