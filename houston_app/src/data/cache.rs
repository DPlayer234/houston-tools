use std::mem;
use std::sync::OnceLock;

use dashmap::DashMap;
use extract_map::ExtractMap;
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
#[derive(Default)]
pub struct Cache {
    current_user: OnceLock<CurrentUser>,
    guilds: DashMap<GuildId, CachedGuild>,
}

#[derive(Default)]
struct CachedGuild {
    channels: ExtractMap<ChannelId, GuildChannel>,
    threads: ExtractMap<ChannelId, GuildChannel>,
}

utils::impl_debug!(struct Cache: { .. });

fn is_thread(kind: ChannelType) -> bool {
    matches!(
        kind,
        ChannelType::NewsThread | ChannelType::PublicThread | ChannelType::PrivateThread
    )
}

impl CachedGuild {
    pub fn get_map_for(&mut self, kind: ChannelType) -> &mut ExtractMap<ChannelId, GuildChannel> {
        if is_thread(kind) {
            &mut self.threads
        } else {
            &mut self.channels
        }
    }
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
    ) -> serenity::Result<GuildChannel> {
        if let Some(channel) = self.guild_channel_(guild_id, channel_id) {
            return Ok(channel);
        }

        self.fetch_channel(http, channel_id).await
    }

    /// Gets the thread with a given ID in a guild.
    ///
    /// If the channel isn't a thread, returns [`None`].
    pub async fn thread_channel(
        &self,
        http: &Http,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> serenity::Result<Option<GuildChannel>> {
        if let Some(thread) = self.thread_channel_(guild_id, channel_id) {
            return Ok(thread);
        }

        let channel = self.fetch_channel(http, channel_id).await?;
        if is_thread(channel.kind) {
            Ok(Some(channel))
        } else {
            Ok(None)
        }
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
    ) -> serenity::Result<GuildChannel> {
        if let Some(channel) = self.super_channel_(guild_id, &mut channel_id) {
            return Ok(channel);
        }

        let mut channel = self.fetch_channel(http, channel_id).await?;
        if let Some(parent_id) = channel.parent_id {
            if is_thread(channel.kind) {
                channel = self.fetch_channel(http, parent_id).await?;
            }
        }

        Ok(channel)
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
                "**?{id}:** channels: {}, threads: {}",
                guild.channels.len(),
                guild.threads.len()
            );
        }

        (!out.is_empty()).then_some(out)
    }

    fn guild_channel_(&self, guild_id: GuildId, channel_id: ChannelId) -> Option<GuildChannel> {
        let guild = self.guilds.get(&guild_id)?;
        let channel = guild.channels.get(&channel_id);
        channel.or_else(|| guild.threads.get(&channel_id)).cloned()
    }

    fn thread_channel_(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> Option<Option<GuildChannel>> {
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
    ) -> Option<GuildChannel> {
        let guild = self.guilds.get(&guild_id)?;
        if let Some(parent_id) = guild.threads.get(channel_id).and_then(|t| t.parent_id) {
            *channel_id = parent_id;
        }

        guild.channels.get(channel_id).cloned()
    }

    async fn fetch_channel(
        &self,
        http: &Http,
        channel_id: ChannelId,
    ) -> serenity::Result<GuildChannel> {
        log::warn!("Cache miss for channel: {channel_id}");

        let channel = http.get_channel(channel_id).await?;
        let channel = channel.guild().ok_or(ModelError::InvalidChannelType)?;
        self.update_channel(channel.clone());
        Ok(channel)
    }

    fn update_channel(&self, channel: GuildChannel) {
        let mut guild = self.guilds.entry(channel.guild_id).or_default();
        guild.get_map_for(channel.kind).insert(channel);
    }

    fn remove_channel(&self, channel: &GuildChannel) {
        if let Some(mut guild) = self.guilds.get_mut(&channel.guild_id) {
            guild.get_map_for(channel.kind).remove(&channel.id);
        }
    }

    fn remove_partial_channel(&self, channel: &PartialGuildChannel) {
        if let Some(mut guild) = self.guilds.get_mut(&channel.guild_id) {
            guild.get_map_for(channel.kind).remove(&channel.id);
        }
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
        self.update_channel(value.channel.clone());
    }
}

impl CacheUpdate<ChannelDeleteEvent> for Cache {
    fn update(&self, value: &ChannelDeleteEvent) {
        self.remove_channel(&value.channel);
    }
}

impl CacheUpdate<ChannelUpdateEvent> for Cache {
    fn update(&self, value: &ChannelUpdateEvent) {
        self.update_channel(value.channel.clone());
    }
}

impl CacheUpdate<GuildCreateEvent> for Cache {
    fn update(&self, value: &GuildCreateEvent) {
        if value.guild.unavailable() {
            return;
        }

        let guild = CachedGuild {
            channels: value.guild.channels.iter().cloned().collect(),
            threads: value.guild.threads.iter().cloned().collect(),
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
        self.update_channel(value.thread.clone());
    }
}

impl CacheUpdate<ThreadDeleteEvent> for Cache {
    fn update(&self, value: &ThreadDeleteEvent) {
        self.remove_partial_channel(&value.thread);
    }
}

impl CacheUpdate<ThreadUpdateEvent> for Cache {
    fn update(&self, value: &ThreadUpdateEvent) {
        self.update_channel(value.thread.clone());
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
                if thread.parent_id.is_some_and(|p| !parents.contains(&p)) {
                    guild.threads.insert(thread);
                }
            }
        }

        for thread in &value.threads {
            guild.threads.insert(thread.clone());
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
