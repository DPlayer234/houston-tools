use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use houston_utils::discord::emoji_equivalent;
use serenity::model::channel::ReactionType;
use serenity::model::id::ParseIdError;
use serenity::small_fixed_array::FixedString;

/// Config-compatible Discord emoji with [`Hash`] and [`Eq`] based on just ID
/// for custom emojis and character for unicode ones.
#[derive(Debug)]
pub struct HEmoji(ReactionType);

impl HEmoji {
    pub fn as_emoji(&self) -> &ReactionType {
        &self.0
    }

    pub fn equivalent_to(&self, other: &ReactionType) -> bool {
        emoji_equivalent(self.as_emoji(), other)
    }
}

impl PartialEq for HEmoji {
    fn eq(&self, other: &Self) -> bool {
        self.equivalent_to(&other.0)
    }
}

impl Eq for HEmoji {}

impl Hash for HEmoji {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self.0 {
            ReactionType::Custom { id, .. } => id.hash(state),
            ReactionType::Unicode(name) => name.hash(state),
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for HEmoji {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for HEmoji {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let emoji = if let Some((name, id)) = s.split_once(':') {
            let id = id.parse()?;
            let name = Some(FixedString::from_str_trunc(name));
            ReactionType::Custom {
                animated: false,
                id,
                name,
            }
        } else {
            ReactionType::Unicode(FixedString::from_str_trunc(s))
        };

        Ok(Self(emoji))
    }
}

impl<'de> serde::Deserialize<'de> for HEmoji {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = HEmoji;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("string for emoji")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse().map_err(|_| E::custom("invalid emoji id"))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

macro_rules! declare_emojis {
    ($($emoji:ident($lit:literal);)*) => {
        $(
            /// Returns this unicode emoji:
            #[doc = $lit]
            pub fn $emoji() -> ReactionType {
                ::houston_utils::discord::unicode_emoji($lit)
            }
        )*
    };
}

declare_emojis! {
    back("⏪");
    left("◀");
    right("▶");
}
