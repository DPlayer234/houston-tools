use extract_map::ExtractKey;

use super::is_thread;
use crate::prelude::*;

/// Minimal cached information about a guild channel.
#[derive(Debug, Clone)]
pub struct CachedChannel {
    pub id: ChannelId,
    #[expect(dead_code, reason = "parallel, useful for debugging")]
    pub kind: ChannelType,
    pub guild_id: GuildId,
    pub nsfw: bool,
}

/// Minimal cached information about a thread.
#[derive(Debug, Clone)]
pub struct CachedThread {
    pub id: ChannelId,
    pub kind: ChannelType,
    pub guild_id: GuildId,
    pub parent_id: ChannelId,
}

/// Cached Channel or Thread
#[derive(Debug, Clone)]
pub enum Ccot {
    Channel(CachedChannel),
    Thread(CachedThread),
}

impl Ccot {
    pub fn guild_id(&self) -> GuildId {
        match self {
            Self::Channel(c) => c.guild_id,
            Self::Thread(t) => t.guild_id,
        }
    }

    pub fn channel(self) -> Option<CachedChannel> {
        match self {
            Self::Channel(c) => Some(c),
            _ => None,
        }
    }

    pub fn thread(self) -> Option<CachedThread> {
        match self {
            Self::Thread(t) => Some(t),
            _ => None,
        }
    }
}

impl ExtractKey<ChannelId> for CachedChannel {
    fn extract_key(&self) -> &ChannelId {
        &self.id
    }
}

impl ExtractKey<ChannelId> for CachedThread {
    fn extract_key(&self) -> &ChannelId {
        &self.id
    }
}

impl From<GuildChannel> for Ccot {
    fn from(value: GuildChannel) -> Self {
        (&value).into()
    }
}

impl From<&GuildChannel> for Ccot {
    fn from(value: &GuildChannel) -> Self {
        if is_thread(value.kind) {
            Self::Thread(value.into())
        } else {
            Self::Channel(value.into())
        }
    }
}

impl From<&GuildChannel> for CachedChannel {
    fn from(value: &GuildChannel) -> Self {
        if is_thread(value.kind) {
            log::warn!("Channel {} is actually a thread.", value.id);
        }

        Self {
            id: value.id,
            kind: value.kind,
            guild_id: value.guild_id,
            nsfw: value.nsfw,
        }
    }
}

impl From<&GuildChannel> for CachedThread {
    fn from(value: &GuildChannel) -> Self {
        if !is_thread(value.kind) {
            log::warn!("Thread {} is actually a channel.", value.id);
        }

        let parent_id = match value.parent_id {
            Some(parent_id) => parent_id,
            None => {
                log::warn!("Thread {} has no parent.", value.id);
                ChannelId::default()
            },
        };

        Self {
            id: value.id,
            kind: value.kind,
            guild_id: value.guild_id,
            parent_id,
        }
    }
}
