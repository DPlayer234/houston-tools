use std::collections::HashMap;

use serenity::small_fixed_array::FixedArray;

use crate::config::HEmoji;
use crate::prelude::*;

pub type Config = HashMap<GenericChannelId, MediaReactChannel>;

fn default_with_threads() -> bool {
    true
}

#[derive(Debug, serde::Deserialize)]
pub struct MediaReactChannel {
    pub emojis: FixedArray<MediaReactEntry>,
    #[serde(default = "default_with_threads")]
    pub with_threads: bool,
}

#[derive(Debug)]
pub struct MediaReactEntry {
    pub emoji: HEmoji,
    pub condition: MediaCheck,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    /// React to messages with links or attachments.
    #[default]
    Content,
    /// React to no messages.
    Never,
    /// React to all messages.
    Always,
}

impl Condition {
    pub fn select(self, f: impl FnOnce() -> bool) -> bool {
        match self {
            Self::Never => false,
            Self::Always => true,
            Self::Content => f(),
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct MediaCheck {
    pub normal: Condition,
    pub forward: Condition,
}

mod serde_impl {
    use std::fmt;

    use serde::Deserialize;
    use serde::de::value::{MapAccessDeserializer, SeqAccessDeserializer, StrDeserializer};
    use serde::de::{Deserializer, Error, MapAccess, SeqAccess, Visitor};

    use super::{Condition, MediaCheck, MediaReactEntry};
    use crate::config::HEmoji;

    impl<'de> Deserialize<'de> for MediaCheck {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            #[derive(Deserialize)]
            struct Basic {
                normal: Condition,
                forward: Condition,
            }

            fn into_real(Basic { normal, forward }: Basic) -> MediaCheck {
                MediaCheck { normal, forward }
            }

            struct MediaCheckVisitor;
            impl<'de> Visitor<'de> for MediaCheckVisitor {
                type Value = MediaCheck;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("`content`, `never`, or `always` or a map")
                }

                fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>,
                {
                    Basic::deserialize(MapAccessDeserializer::new(map)).map(into_real)
                }

                fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    Basic::deserialize(SeqAccessDeserializer::new(seq)).map(into_real)
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Condition::deserialize(StrDeserializer::new(v)).map(|c| MediaCheck {
                        normal: c,
                        forward: c,
                    })
                }
            }

            deserializer.deserialize_any(MediaCheckVisitor)
        }
    }

    impl<'de> Deserialize<'de> for MediaReactEntry {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            #[derive(Deserialize)]
            struct Full {
                emoji: HEmoji,
                #[serde(default)]
                condition: MediaCheck,
            }

            fn into_real(Full { emoji, condition }: Full) -> MediaReactEntry {
                MediaReactEntry { emoji, condition }
            }

            struct MediaCheckVisitor;
            impl<'de> Visitor<'de> for MediaCheckVisitor {
                type Value = MediaReactEntry;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("an emoji string or a map")
                }

                fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>,
                {
                    Full::deserialize(MapAccessDeserializer::new(map)).map(into_real)
                }

                fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    Full::deserialize(SeqAccessDeserializer::new(seq)).map(into_real)
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    HEmoji::deserialize(StrDeserializer::new(v)).map(|emoji| MediaReactEntry {
                        emoji,
                        condition: MediaCheck::default(),
                    })
                }
            }

            deserializer.deserialize_any(MediaCheckVisitor)
        }
    }
}
