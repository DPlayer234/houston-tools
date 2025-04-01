use std::collections::{HashMap, HashSet};
use std::ops::DerefMut;
use std::sync::Arc;

use arc_swap::ArcSwapOption;
use dashmap::DashMap;
use extract_map::ExtractMap;
use serenity::http::Http;

use crate::fmt::discord::id_suffix;
use crate::prelude::*;

mod event_handler;
mod model;

pub use model::{CachedChannel, CachedThread, Ccot};

/// Provides a simple application-specific cache for Discord state.
///
/// Currently, this only serves to cache channels and threads for guilds, as
/// well as the current user.
///
/// Requires the [`GatewayIntents::GUILDS`] intent.
//
// CMBK: edge-case "loses access to channels with threads"
// the threads in question will stay in the cache, not sure how to solve that well
#[derive(Default)]
pub struct Cache {
    current_user: ArcSwapOption<CurrentUser>,
    guilds: DashMap<GuildId, CachedGuild>,
}

#[derive(Default)]
struct CachedGuild {
    channels: ExtractMap<ChannelId, CachedChannel>,
    threads: ExtractMap<ChannelId, CachedThread>,
    /// Tracks threads in a channel. The key is the parent channel ID.
    threads_in: HashMap<ChannelId, HashSet<ChannelId>>,
}

utils::impl_debug!(struct Cache: { .. });

fn is_thread(kind: ChannelType) -> bool {
    matches!(
        kind,
        ChannelType::NewsThread | ChannelType::PublicThread | ChannelType::PrivateThread
    )
}

/// API for accessing the cache.
impl Cache {
    /// Gets the cached current bot user.
    pub fn current_user(&self) -> Result<Arc<CurrentUser>> {
        self.current_user
            .load_full()
            .context("current user not loaded")
    }

    /// Gets the guild channel with a given ID in a guild.
    ///
    /// The result may either be a normal channel or thread.
    #[expect(dead_code, reason = "... probably useful")]
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

        if matches!(self.guilds.len(), 0 | 11..) {
            return None;
        }

        let mut out = String::new();
        for entry in &self.guilds {
            let guild = entry.value();
            let id = id_suffix(*entry.key());
            writeln_str!(
                out,
                "**{id}:** channels: {}, threads: {}",
                guild.channels.len(),
                guild.threads.len()
            );
        }

        Some(out)
    }

    fn guild_channel_(&self, guild_id: GuildId, channel_id: ChannelId) -> Option<Ccot> {
        let g = self.guilds.get(&guild_id)?;

        if let Some(channel) = g.channels.get(&channel_id) {
            return Some(Ccot::Channel(channel.clone()));
        }

        g.threads.get(&channel_id).map(|t| Ccot::Thread(t.clone()))
    }

    /// Returns:
    /// - `Some(None)` for normal channels
    /// - `Some(Some(_))` for threads
    /// - `None` for cache misses
    fn thread_channel_(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> Option<Option<CachedThread>> {
        let guild = self.guilds.get(&guild_id)?;

        if guild.channels.get(&channel_id).is_some() {
            return Some(None);
        }

        if let Some(thread) = guild.threads.get(&channel_id) {
            return Some(Some(thread.clone()));
        }

        None
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

    /// Fetches a channel/thread via HTTP and caches it.
    async fn fetch_channel(&self, http: &Http, channel_id: ChannelId) -> serenity::Result<Ccot> {
        let channel = http.get_channel(channel_id).await?;
        let channel = channel.guild().ok_or(ModelError::InvalidChannelType)?;

        let GuildChannel { id, name, .. } = &channel;
        log::warn!("Cache miss for channel `{name}` ({id}).");

        let channel = Ccot::from(&channel);
        self.update_channel(channel.clone());
        Ok(channel)
    }

    /// Adds or updates a channel or thread in the cache.
    fn update_channel(&self, channel: Ccot) {
        let mut guild = self.guilds.entry(channel.guild_id()).or_default();
        match channel {
            Ccot::Channel(c) => _ = guild.channels.insert(c),
            Ccot::Thread(t) => _ = guild.threads.insert(t),
        }
    }

    /// Gets or inserts a cached guild and returns a handle to it.
    fn insert_guild(&self, guild_id: GuildId) -> impl DerefMut<Target = CachedGuild> {
        self.guilds.entry(guild_id).or_default()
    }

    /// Gets the ID of the current user.
    fn current_user_id(&self) -> Option<UserId> {
        self.current_user.load().as_deref().map(|u| u.id)
    }

    /// Replaces the cached current user info.
    fn set_current_user(&self, user: CurrentUser) {
        self.current_user.store(Some(Arc::new(user)));
    }
}

impl CachedGuild {
    /// Adds a thread to the guild cache.
    fn add_thread(&mut self, thread: CachedThread) {
        // track the thread for the parent channel
        self.threads_in
            .entry(thread.parent_id)
            .or_default()
            .insert(thread.id);

        self.threads.insert(thread);
    }

    /// Removes a thread from the guild cache.
    fn remove_thread(&mut self, parent_id: ChannelId, thread_id: ChannelId) {
        self.threads.remove(&thread_id);

        // remove the thread from the parent channel set
        if let Some(set) = self.threads_in.get_mut(&parent_id) {
            set.remove(&thread_id);
        }
    }

    /// Remove all threads associated with a given channel.
    fn remove_associated_threads(&mut self, parent_id: ChannelId) {
        let thread_ids = self.threads_in.remove(&parent_id).unwrap_or_default();

        for thread_id in thread_ids {
            self.threads.remove(&thread_id);
        }
    }
}
