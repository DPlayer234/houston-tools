use std::ops::DerefMut;

use arc_swap::ArcSwapOption;
use dashmap::DashMap;
use serenity::http::Http;

use crate::fmt::discord::id_suffix;
use crate::prelude::*;

mod event_handler;
mod model;

pub use event_handler::CacheUpdateHandler;
pub use model::{CachedChannel, CachedGuild, CachedThread, Ccot};

/// Provides a simple application-specific cache for Discord state.
///
/// Currently, this only serves to cache channels and threads for guilds, as
/// well as the current user.
#[derive(Default)]
pub struct Cache {
    current_user: ArcSwapOption<CurrentUser>,
    guilds: DashMap<GuildId, CachedGuild>,
}

utils::impl_debug!(struct Cache: { .. });

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
        channel_id: GenericChannelId,
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
        thread_id: ThreadId,
    ) -> serenity::Result<Option<CachedThread>> {
        if let Some(thread) = self.thread_channel_(guild_id, thread_id) {
            return Ok(thread);
        }

        let channel = self.fetch_channel(http, thread_id.widen()).await?;
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
        mut channel_id: GenericChannelId,
    ) -> serenity::Result<CachedChannel> {
        if let Some(channel) = self.super_channel_(guild_id, &mut channel_id) {
            return Ok(channel);
        }

        match self.fetch_channel(http, channel_id).await? {
            Ccot::Channel(channel) => Ok(channel),
            Ccot::Thread(thread) => {
                let channel = self.fetch_channel(http, thread.parent_id.widen()).await?;
                Ok(channel.channel().ok_or(ModelError::InvalidChannelType)?)
            },
        }
    }

    /// Provides a string with cache statistics.
    pub fn stats(&self) -> Option<String> {
        use utils::text::WriteStr as _;

        if matches!(self.guilds.len(), 0 | 11..) {
            return None;
        }

        let mut out = String::new();
        for entry in &self.guilds {
            let guild = entry.value();
            let id = id_suffix(*entry.key());
            writeln!(
                out,
                "**{id}:** channels: {}, threads: {}",
                guild.channels.len(),
                guild.threads.len()
            );
        }

        Some(out)
    }

    fn guild_channel_(&self, guild_id: GuildId, channel_id: GenericChannelId) -> Option<Ccot> {
        let guild = self.guilds.get(&guild_id)?;

        if let Some(channel) = guild.channels.get(&channel_id.expect_channel()) {
            return Some(Ccot::Channel(channel.clone()));
        }

        guild
            .threads
            .get(&channel_id.expect_thread())
            .map(|t| Ccot::Thread(t.clone()))
    }

    /// Returns:
    /// - `Some(None)` for normal channels
    /// - `Some(Some(_))` for threads
    /// - `None` for cache misses
    fn thread_channel_(
        &self,
        guild_id: GuildId,
        thread_id: ThreadId,
    ) -> Option<Option<CachedThread>> {
        let guild = self.guilds.get(&guild_id)?;

        // to not cause a cache miss if this is called with a normal channel id when
        // looking up from `GenericChannelId`, check the normal channels first
        let as_channel_id = thread_id.widen().expect_channel();
        if guild.channels.get(&as_channel_id).is_some() {
            return Some(None);
        }

        if let Some(thread) = guild.threads.get(&thread_id) {
            return Some(Some(thread.clone()));
        }

        None
    }

    fn super_channel_(
        &self,
        guild_id: GuildId,
        channel_id: &mut GenericChannelId,
    ) -> Option<CachedChannel> {
        let guild = self.guilds.get(&guild_id)?;
        if let Some(thread) = guild.threads.get(&channel_id.expect_thread()) {
            *channel_id = thread.parent_id.widen();
        }

        guild.channels.get(&channel_id.expect_channel()).cloned()
    }

    /// Fetches a channel/thread via HTTP and caches it.
    #[cold]
    async fn fetch_channel(
        &self,
        http: &Http,
        channel_id: GenericChannelId,
    ) -> serenity::Result<Ccot> {
        let channel = http.get_channel(channel_id).await?;

        fn warn_miss(label: &str, name: &str, id: u64) {
            log::warn!("Cache miss for {label} `{name}` ({id}).");
        }

        macro_rules! handle {
            ($channel:expr, $Ty:ty, $cache:ident, $ccot:ident) => {{
                warn_miss(stringify!($ccot), &$channel.base.name, $channel.id.get());

                let c = <$Ty>::from(&$channel);
                self.insert_guild(c.guild_id).$cache.insert(c.clone());

                Ok(Ccot::$ccot(c))
            }};
        }

        match channel {
            Channel::Guild(channel) => handle!(channel, CachedChannel, channels, Channel),
            Channel::GuildThread(thread) => handle!(thread, CachedThread, threads, Thread),
            _ => Err(ModelError::InvalidChannelType.into()),
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
