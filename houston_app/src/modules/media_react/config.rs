use std::collections::HashMap;

use crate::config::HEmoji;
use crate::prelude::*;

pub type Config = HashMap<ChannelId, Vec<MediaReactEntry>>;

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
    #[inline]
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

impl From<MediaCheckDe> for MediaCheck {
    fn from(value: MediaCheckDe) -> Self {
        let (normal, forward) = match value {
            MediaCheckDe::Specific { normal, forward } => (normal, forward),
            MediaCheckDe::Same(condition) => (condition, condition),
        };

        Self { normal, forward }
    }
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

#[derive(Debug, serde::Deserialize)]
pub struct MediaReactEntry {
    pub emoji: HEmoji,

    #[serde(default)]
    pub condition: MediaCheck,
}
