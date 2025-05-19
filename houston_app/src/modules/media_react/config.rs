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

#[derive(Debug, serde::Deserialize)]
#[serde(from = "MediaReactEntryDe")]
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

#[derive(Default, Debug, Clone, Copy, serde::Deserialize)]
#[serde(from = "MediaCheckDe")]
pub struct MediaCheck {
    pub normal: Condition,
    pub forward: Condition,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(untagged)]
enum MediaCheckDe {
    Specific {
        #[serde(default)]
        normal: Condition,
        #[serde(default)]
        forward: Condition,
    },
    Same(Condition),
}

impl From<MediaCheckDe> for MediaCheck {
    fn from(value: MediaCheckDe) -> Self {
        let (normal, forward) = match value {
            MediaCheckDe::Specific { normal, forward } => (normal, forward),
            MediaCheckDe::Same(condition) => (condition, condition),
        };

        Self { normal, forward }
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum MediaReactEntryDe {
    Emoji(HEmoji),
    Full {
        emoji: HEmoji,
        #[serde(default)]
        condition: MediaCheck,
    },
}

impl From<MediaReactEntryDe> for MediaReactEntry {
    fn from(value: MediaReactEntryDe) -> Self {
        let (emoji, condition) = match value {
            MediaReactEntryDe::Emoji(emoji) => (emoji, MediaCheck::default()),
            MediaReactEntryDe::Full { emoji, condition } => (emoji, condition),
        };

        Self { emoji, condition }
    }
}
