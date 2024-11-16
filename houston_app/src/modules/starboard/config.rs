use std::collections::HashMap;
use std::fmt;

use bson::doc;

use crate::prelude::*;

pub type Config = HashMap<GuildId, StarboardGuild>;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, serde::Deserialize)]
pub struct StarboardGuild {
    pub boards: Vec<StarboardEntry>,
}

#[derive(Debug, serde::Deserialize)]
pub struct StarboardEntry {
    pub id: BoardId,
    pub name: String,
    pub channel: ChannelId,
    pub emoji: StarboardEmoji,
    pub reacts: u8,
    #[serde(default = "Vec::new")]
    pub notices: Vec<String>,
    #[serde(default)]
    pub cash_gain: i8,
}

#[derive(Debug)]
pub struct StarboardEmoji(ReactionType);

impl StarboardEmoji {
    pub fn as_emoji(&self) -> &ReactionType {
        &self.0
    }

    pub fn name(&self) -> &str {
        match self.as_emoji() {
            ReactionType::Custom { name, .. } => name.as_ref().expect("always set").as_str(),
            ReactionType::Unicode(unicode) => unicode.as_str(),
            _ => panic!("never set to invalid"),
        }
    }

    pub fn equivalent_to(&self, reaction: &ReactionType) -> bool {
        match (self.as_emoji(), reaction) {
            (ReactionType::Custom { id: self_id, .. }, ReactionType::Custom { id: other_id, .. }) => self_id == other_id,
            (ReactionType::Unicode(self_name), ReactionType::Unicode(other_name)) => self_name == other_name,
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

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = StarboardEmoji;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("expected string for emoji")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let emoji = if let Some((id, name)) = v.split_once(':') {
                    let id = id.parse::<EmojiId>().map_err(|_| E::custom("invalid emoji id"))?;
                    ReactionType::Custom { animated: false, id, name: Some(FixedString::from_str_trunc(name)) }
                } else {
                    ReactionType::Unicode(FixedString::from_str_trunc(v))
                };

                Ok(StarboardEmoji(emoji))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}
