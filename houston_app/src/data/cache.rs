use std::mem;
use std::ops::DerefMut;
use std::sync::OnceLock;

use dashmap::DashMap;
use extract_map::{ExtractKey, ExtractMap};
use serenity::futures::future::{BoxFuture, always_ready};
use serenity::gateway::client::{Context, RawEventHandler};
use serenity::http::Http;

use crate::prelude::*;

/// Provides a simple application-specific cache for Discord state.
///
/// Currently, this only serves to cache channels and threads for guilds, as
/// well as the current user.
///
/// Requires the [`GatewayIntents::GUILDS`] intent.
//
// CMBK: edge-case "looses access to channels with threads"
// the threads in question will stay in the cache, not sure how to solve that well
#[derive(Default)]
pub struct Cache {
    current_user: OnceLock<CurrentUser>,
    guilds: DashMap<GuildId, CachedGuild>,
}

#[derive(Default)]
struct CachedGuild {
    channels: ExtractMap<ChannelId, CachedChannel>,
    threads: ExtractMap<ChannelId, CachedThread>,
}

utils::impl_debug!(struct Cache: { .. });

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

fn is_thread(kind: ChannelType) -> bool {
    matches!(
        kind,
        ChannelType::NewsThread | ChannelType::PublicThread | ChannelType::PrivateThread
    )
}

impl Cache {
    /// Gets the cached current bot user.
    pub fn current_user(&self) -> Result<&CurrentUser> {
        self.current_user.get().context("current user not loaded")
    }

    /// Gets the guild channel with a given ID in a guild.
    ///
    /// The result may either be a normal channel or thread.
    #[allow(dead_code, reason = "... probably useful")]
    pub async fn guild_channel(
        &self,
        http: &Http,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> serenity::Result<Ccot> {
        if let Some(channel) = self.guild_channel_(guild_id, channel_id) {
            return Ok(channel);
        }

        let channel = self.fetch_channel(http, channel_id).await?;
        Ok(channel)
    }

    /// Gets the thread with a given ID in a guild.
    ///
    /// If the channel isn't a thread, returns [`None`].
    pub async fn thread_channel(
        &self,
        http: &Http,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> serenity::Result<Option<CachedThread>> {
        if let Some(thread) = self.thread_channel_(guild_id, channel_id) {
            return Ok(thread);
        }

        let channel = self.fetch_channel(http, channel_id).await?;
        Ok(channel.thread())
    }

    /// Gets the "super" guild channel for a given channel ID in a guild.
    ///
    /// That is to say, if `channel_id` represents a thread, this will return
    /// its parent. Otherwise, returns the channel with the given ID.
    pub async fn super_channel(
        &self,
        http: &Http,
        guild_id: GuildId,
        mut channel_id: ChannelId,
    ) -> serenity::Result<CachedChannel> {
        if let Some(channel) = self.super_channel_(guild_id, &mut channel_id) {
            return Ok(channel);
        }

        match self.fetch_channel(http, channel_id).await? {
            Ccot::Channel(channel) => Ok(channel),
            Ccot::Thread(thread) => {
                let channel = self.fetch_channel(http, thread.parent_id).await?;
                Ok(channel.channel().ok_or(ModelError::InvalidChannelType)?)
            },
        }
    }

    /// Provides a string with cache statistics.
    pub fn stats(&self) -> Option<String> {
        use utils::text::write_str::*;

        let mut out = String::new();

        for entry in &self.guilds {
            let guild = entry.value();
            let id = entry.key().get() % 1_000_000;
            writeln_str!(
                out,
                "**?{id:06}:** channels: {}, threads: {}",
                guild.channels.len(),
                guild.threads.len()
            );
        }

        (!out.is_empty()).then_some(out)
    }

    fn guild_channel_(&self, guild_id: GuildId, channel_id: ChannelId) -> Option<Ccot> {
        let g = self.guilds.get(&guild_id)?;

        if let Some(channel) = g.channels.get(&channel_id) {
            return Some(Ccot::Channel(channel.clone()));
        }

        g.threads.get(&channel_id).map(|t| Ccot::Thread(t.clone()))
    }

    fn thread_channel_(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> Option<Option<CachedThread>> {
        let guild = self.guilds.get(&guild_id)?;
        let channel = guild.channels.get(&channel_id);
        if channel.is_some() {
            return None;
        }

        Some(guild.threads.get(&channel_id).cloned())
    }

    fn super_channel_(
        &self,
        guild_id: GuildId,
        channel_id: &mut ChannelId,
    ) -> Option<CachedChannel> {
        let guild = self.guilds.get(&guild_id)?;
        if let Some(thread) = guild.threads.get(channel_id) {
            *channel_id = thread.parent_id;
        }

        guild.channels.get(channel_id).cloned()
    }

    async fn fetch_channel(&self, http: &Http, channel_id: ChannelId) -> serenity::Result<Ccot> {
        log::warn!("Cache miss for channel: {channel_id}");

        let channel = http.get_channel(channel_id).await?;
        let channel = channel.guild().ok_or(ModelError::InvalidChannelType)?;
        let channel = Ccot::from(&channel);
        self.update_channel(channel.clone());
        Ok(channel)
    }

    fn update_channel(&self, channel: Ccot) {
        let mut guild = self.guilds.entry(channel.guild_id()).or_default();
        match channel {
            Ccot::Channel(c) => _ = guild.channels.insert(c),
            Ccot::Thread(t) => _ = guild.threads.insert(t),
        }
    }

    fn insert_guild(&self, guild_id: GuildId) -> impl DerefMut<Target = CachedGuild> {
        self.guilds.entry(guild_id).or_default()
    }

    fn remove_thread_if_private(&self, guild_id: GuildId, thread_id: ChannelId) {
        if let Some(mut guild) = self.guilds.get_mut(&guild_id) {
            if let Some(thread) = guild.threads.get(&thread_id) {
                if thread.kind == ChannelType::PrivateThread {
                    guild.threads.remove(&thread_id);
                }
            }
        }
    }

    fn update_event(&self, event: &Event) {
        match event {
            Event::Ready(event) => self.update(event),
            Event::ChannelCreate(event) => self.update(event),
            Event::ChannelDelete(event) => self.update(event),
            Event::ChannelUpdate(event) => self.update(event),
            Event::GuildCreate(event) => self.update(event),
            Event::GuildDelete(event) => self.update(event),
            Event::ThreadCreate(event) => self.update(event),
            Event::ThreadUpdate(event) => self.update(event),
            Event::ThreadDelete(event) => self.update(event),
            Event::ThreadListSync(event) => self.update(event),
            Event::ThreadMembersUpdate(event) => self.update(event),
            _ => {},
        }
    }
}

impl RawEventHandler for Cache {
    fn raw_event<'s: 'f, 'e: 'f, 'f>(&'s self, _ctx: Context, ev: &'e Event) -> BoxFuture<'f, ()> {
        self.update_event(ev);
        Box::pin(always_ready(|| ()))
    }
}

trait CacheUpdate<T> {
    fn update(&self, value: &T);
}

impl CacheUpdate<ReadyEvent> for Cache {
    fn update(&self, value: &ReadyEvent) {
        self.current_user.get_or_init(|| value.ready.user.clone());
        self.guilds.clear();
    }
}

impl CacheUpdate<ChannelCreateEvent> for Cache {
    fn update(&self, value: &ChannelCreateEvent) {
        let mut guild = self.insert_guild(value.channel.guild_id);
        guild.channels.insert((&value.channel).into());
    }
}

impl CacheUpdate<ChannelDeleteEvent> for Cache {
    fn update(&self, value: &ChannelDeleteEvent) {
        if let Some(mut guild) = self.guilds.get_mut(&value.channel.guild_id) {
            guild.channels.remove(&value.channel.id);
        }
    }
}

impl CacheUpdate<ChannelUpdateEvent> for Cache {
    fn update(&self, value: &ChannelUpdateEvent) {
        let mut guild = self.insert_guild(value.channel.guild_id);
        guild.channels.insert((&value.channel).into());
    }
}

impl CacheUpdate<GuildCreateEvent> for Cache {
    fn update(&self, value: &GuildCreateEvent) {
        if value.guild.unavailable() {
            return;
        }

        let Guild {
            channels, threads, ..
        } = &value.guild;

        let guild = CachedGuild {
            channels: channels.iter().map(CachedChannel::from).collect(),
            threads: threads.iter().map(CachedThread::from).collect(),
        };

        self.guilds.insert(value.guild.id, guild);
    }
}

impl CacheUpdate<GuildDeleteEvent> for Cache {
    fn update(&self, value: &GuildDeleteEvent) {
        self.guilds.remove(&value.guild.id);
    }
}

impl CacheUpdate<ThreadCreateEvent> for Cache {
    fn update(&self, value: &ThreadCreateEvent) {
        let mut guild = self.insert_guild(value.thread.guild_id);
        guild.threads.insert((&value.thread).into());
    }
}

impl CacheUpdate<ThreadDeleteEvent> for Cache {
    fn update(&self, value: &ThreadDeleteEvent) {
        if let Some(mut guild) = self.guilds.get_mut(&value.thread.guild_id) {
            guild.threads.remove(&value.thread.id);
        }
    }
}

impl CacheUpdate<ThreadUpdateEvent> for Cache {
    fn update(&self, value: &ThreadUpdateEvent) {
        let mut guild = self.insert_guild(value.thread.guild_id);
        guild.threads.insert((&value.thread).into());
    }
}

impl CacheUpdate<ThreadListSyncEvent> for Cache {
    fn update(&self, value: &ThreadListSyncEvent) {
        let mut guild = self.guilds.entry(value.guild_id).or_default();

        // move out the old thread cache
        // if no `channel_ids` are specified, it needs to be cleared anyways
        let threads = mem::take(&mut guild.threads);

        if let Some(parents) = &value.channel_ids {
            // insert all threads back in which aren't updated
            for thread in threads {
                if !parents.contains(&thread.parent_id) {
                    guild.threads.insert(thread);
                }
            }
        }

        for thread in &value.threads {
            guild.threads.insert(thread.into());
        }
    }
}

impl CacheUpdate<ThreadMembersUpdateEvent> for Cache {
    fn update(&self, value: &ThreadMembersUpdateEvent) {
        let Some(current_user) = self.current_user.get() else {
            log::warn!("Current User is unset.");
            return;
        };

        if value.removed_member_ids.contains(&current_user.id) {
            self.remove_thread_if_private(value.guild_id, value.id);
        }
    }
}
