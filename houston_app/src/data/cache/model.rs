use extract_map::ExtractKey;

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
    pub id: ThreadId,
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

impl ExtractKey<ThreadId> for CachedThread {
    fn extract_key(&self) -> &ThreadId {
        &self.id
    }
}

impl From<&GuildChannel> for CachedChannel {
    fn from(value: &GuildChannel) -> Self {
        Self {
            id: value.id,
            kind: value.base.kind,
            guild_id: value.base.guild_id,
            nsfw: value.nsfw,
        }
    }
}

impl From<&GuildThread> for CachedThread {
    fn from(value: &GuildThread) -> Self {
        Self {
            id: value.id,
            kind: value.base.kind,
            guild_id: value.base.guild_id,
            parent_id: value.parent_id,
        }
    }
}
