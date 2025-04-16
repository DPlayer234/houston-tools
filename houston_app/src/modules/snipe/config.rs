use std::collections::{HashMap, VecDeque};
use std::num::NonZero;
use std::sync::RwLock;

use chrono::TimeDelta;
use serenity::small_fixed_array::FixedString;

use crate::helper::time::serde_time_delta;
use crate::prelude::*;
use crate::slashies::args::SlashUser;

fn default_max_age() -> TimeDelta {
    const { TimeDelta::minutes(5) }
}

fn default_max_cache_size() -> NonZero<usize> {
    const { NonZero::new(64).unwrap() }
}

pub type Config = HashMap<GuildId, GuildConfig>;

#[derive(Debug, serde::Deserialize)]
pub struct GuildConfig {
    #[serde(with = "serde_time_delta", default = "default_max_age")]
    pub max_age: TimeDelta,
    #[serde(default = "default_max_cache_size")]
    pub max_cache_size: NonZero<usize>,

    #[serde(skip)]
    pub state: RwLock<GuildState>,
}

#[derive(Debug, Default)]
pub struct GuildState {
    /// The received messages, including ones already deleted.
    ///
    /// These may be slightly ouf of order.
    pub messages: VecDeque<SnipedMessage>,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, Default)]
    struct SnipedMessageFlags: u8 {
        const DELETED = 0x1;
        const ATTACHMENTS = 0x2;
    }
}

#[derive(Debug, Clone)]
pub struct SnipedMessage {
    pub id: MessageId,
    pub channel_id: GenericChannelId,
    pub author: SnipedAuthor,
    pub content: FixedString<u16>,
    pub timestamp: Timestamp,
    flags: SnipedMessageFlags,
}

#[derive(Debug, Clone)]
pub struct SnipedAuthor {
    pub display_name: FixedString<u8>,
    pub avatar_url: FixedString<u8>,
}

impl SnipedMessage {
    pub fn new(msg: &Message) -> Self {
        let author = SlashUser::from_message(msg);
        let author = SnipedAuthor {
            display_name: FixedString::from_str_trunc(author.display_name()),
            avatar_url: FixedString::from_string_trunc(author.face()),
        };

        let mut flags = SnipedMessageFlags::empty();
        if !msg.attachments.is_empty() {
            flags.insert(SnipedMessageFlags::ATTACHMENTS);
        }

        Self {
            id: msg.id,
            channel_id: msg.channel_id,
            author,
            content: msg.content.clone(),
            timestamp: msg.timestamp,
            flags,
        }
    }

    pub fn update(&mut self, msg: &Message) {
        self.content.clone_from(&msg.content);
        self.flags
            .set(SnipedMessageFlags::ATTACHMENTS, !msg.attachments.is_empty());
    }

    pub fn attachments(&self) -> bool {
        self.flags.contains(SnipedMessageFlags::ATTACHMENTS)
    }

    pub fn deleted(&self) -> bool {
        self.flags.contains(SnipedMessageFlags::DELETED)
    }

    pub(super) fn mark_deleted(&mut self) {
        self.flags.insert(SnipedMessageFlags::DELETED);
    }
}

impl GuildState {
    pub fn get_message_mut(&mut self, message_id: MessageId) -> Option<&mut SnipedMessage> {
        self.messages.iter_mut().find(move |m| m.id == message_id)
    }

    pub fn take_last<F>(&mut self, mut f: F) -> Option<SnipedMessage>
    where
        F: FnMut(&SnipedMessage) -> bool,
    {
        // find the last index of a message matching the predicate
        let mut iter = self.messages.iter().enumerate();
        let (index, _) = iter.rfind(move |(_, m)| f(m))?;
        self.messages.remove(index)
    }
}
