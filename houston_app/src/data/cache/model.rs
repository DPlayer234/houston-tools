use std::collections::{HashMap, HashSet};

use extract_map::{ExtractKey, ExtractMap};

use crate::prelude::*;

/// Minimal cached information about a guild channel.
#[derive(Debug, Clone)]
#[must_use]
pub struct CachedChannel {
    pub id: ChannelId,
    #[expect(dead_code, reason = "parallel, useful for debugging")]
    pub kind: ChannelType,
    pub guild_id: GuildId,
    pub nsfw: bool,
}

/// Minimal cached information about a thread.
#[derive(Debug, Clone)]
#[must_use]
pub struct CachedThread {
    pub id: ThreadId,
    pub kind: ChannelType,
    pub guild_id: GuildId,
    pub parent_id: ChannelId,
}

/// Cached Channel or Thread
#[derive(Debug, Clone)]
#[must_use]
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

#[derive(Default)]
#[must_use]
pub struct CachedGuild {
    pub channels: ExtractMap<ChannelId, CachedChannel>,
    pub threads: ExtractMap<ThreadId, CachedThread>,
    /// Tracks threads in a channel. The key is the parent channel ID.
    pub threads_in: HashMap<ChannelId, HashSet<ThreadId>>,
}

impl CachedGuild {
    /// Adds a thread to the guild cache.
    pub fn add_thread(&mut self, thread: CachedThread) {
        // track the thread for the parent channel
        self.threads_in
            .entry(thread.parent_id)
            .or_default()
            .insert(thread.id);

        self.threads.insert(thread);
    }

    /// Removes a thread from the guild cache.
    pub fn remove_thread(&mut self, parent_id: ChannelId, thread_id: ThreadId) {
        self.threads.remove(&thread_id);

        // remove the thread from the parent channel set
        if let Some(set) = self.threads_in.get_mut(&parent_id) {
            set.remove(&thread_id);
        }
    }

    /// Remove all threads associated with a given channel.
    pub fn remove_associated_threads(&mut self, parent_id: ChannelId) {
        if let Some(thread_ids) = self.threads_in.remove(&parent_id) {
            for thread_id in thread_ids {
                self.threads.remove(&thread_id);
            }
        }
    }
}

impl From<&Guild> for CachedGuild {
    fn from(value: &Guild) -> Self {
        let mut output = Self {
            channels: value.channels.iter().map(CachedChannel::from).collect(),
            threads: value.threads.iter().map(CachedThread::from).collect(),
            threads_in: HashMap::new(),
        };

        // associate existing threads
        for thread in &output.threads {
            output
                .threads_in
                .entry(thread.parent_id)
                .or_default()
                .insert(thread.id);
        }

        output
    }
}
