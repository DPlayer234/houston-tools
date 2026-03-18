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
    #[serde(alias = "emoji", with = "multi_emojis")]
    pub emojis: FixedArray<HEmoji>,
    pub reacts: u32,
    #[serde(default = "FixedArray::new")]
    pub notices: FixedArray<FixedString>,
    #[serde(default, alias = "cash")]
    pub cash_gain: i32,
    #[serde(default, alias = "cash_pin")]
    pub cash_pin_gain: i32,
}

impl StarboardEntry {
    pub fn emoji(&self) -> &HEmoji {
        self.emojis
            .first()
            .expect("starboard emojis should never be empty")
    }

    pub fn has_emoji(&self, emoji: &ReactionType) -> bool {
        self.emojis.iter().any(|e| e.equivalent_to(emoji))
    }

    pub fn any_cash_gain(&self) -> bool {
        self.cash_gain != 0 || self.cash_pin_gain != 0
    }
}

/// Allows accepting either a single emoji or an array of emojis.
///
/// I.e. both of these are valid and deserialize the same:
/// - `emoji = "hello:12345"`
/// - `emojis = ["hello:12345"]`
///
/// Also enforces having at least one emoji.
mod multi_emojis {
    use std::fmt;

    use serde::de::value::{SeqAccessDeserializer, StrDeserializer};
    use serde::de::{Deserialize as _, Deserializer, Error, SeqAccess, Visitor};
    use serenity::small_fixed_array::FixedArray;

    use crate::config::HEmoji;

    struct HEmojiListVisitor;
    impl<'de> Visitor<'de> for HEmojiListVisitor {
        type Value = FixedArray<HEmoji>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an emoji or a sequence of emojis")
        }

        fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let array = FixedArray::deserialize(SeqAccessDeserializer::new(seq))?;
            if !array.is_empty() {
                Ok(array)
            } else {
                Err(A::Error::custom("emoji list cannot be empty"))
            }
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            HEmoji::deserialize(StrDeserializer::new(v)).map(|e| {
                (Box::new([e]) as Box<[_]>)
                    .try_into()
                    .expect("single element should fit into `FixedArray`")
            })
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<FixedArray<HEmoji>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(HEmojiListVisitor)
    }
}
