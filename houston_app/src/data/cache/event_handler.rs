use std::collections::HashMap;

use houston_utils::futures::noop_future;
use serenity::futures::future::BoxFuture;
use serenity::gateway::client::{Context, RawEventHandler};

use super::model::*;
use super::{Cache, CachedGuild};
use crate::prelude::*;

/// [`RawEventHandler`] that updates the cache stored in the [`HContextData`].
///
/// Requires the [`GatewayIntents::GUILDS`] intent for channels and threads. If
/// not set, the channel/thread accessors may provide stale data or make network
/// requests.
//
// CMBK: edge-case "loses access to channels with threads"
// the threads in question will stay in the cache, not sure how to solve that
pub struct CacheUpdateHandler;

// Manual `async_trait` impl to avoid allocation.
impl RawEventHandler for CacheUpdateHandler {
    fn raw_event<'s: 'f, 'e: 'f, 'f>(&'s self, ctx: Context, ev: &'e Event) -> BoxFuture<'f, ()> {
        update_event(&ctx, ev);
        noop_future()
    }
}

/// Updates the cache held by the context with an event.
fn update_event(ctx: &Context, event: &Event) {
    let cache = || ctx.data_ref::<HContextData>().cache();
    match event {
        Event::Ready(event) => cache().update_ready(event),
        Event::ChannelCreate(event) => cache().update_channel_create(event),
        Event::ChannelDelete(event) => cache().update_channel_delete(event),
        Event::ChannelUpdate(event) => cache().update_channel_update(event),
        Event::GuildCreate(event) => cache().update_guild_create(event),
        Event::GuildDelete(event) => cache().update_guild_delete(event),
        Event::UserUpdate(event) => cache().update_user_update(event),
        Event::ThreadCreate(event) => cache().update_thread_create(event),
        Event::ThreadUpdate(event) => cache().update_thread_update(event),
        Event::ThreadDelete(event) => cache().update_thread_delete(event),
        Event::ThreadListSync(event) => cache().update_thread_list_sync(event),
        Event::ThreadMembersUpdate(event) => cache().update_thread_members_update(event),
        _ => {},
    }
}

/// Internal methods to update the cache based on received events.
impl Cache {
    /// Removes a thread from the cache, if it is a private thread.
    fn remove_thread_if_private(&self, guild_id: GuildId, thread_id: ThreadId) {
        if let Some(mut guild) = self.guilds.get_mut(&guild_id)
            && let Some(thread) = guild.threads.get(&thread_id)
            && thread.kind == ChannelType::PrivateThread
        {
            guild.threads.remove(&thread_id);
        }
    }
}

/// The actual update implementations for each relevant event struct.
///
/// Split out here just for clarity.
impl Cache {
    fn update_ready(&self, value: &ReadyEvent) {
        self.set_current_user(value.ready.user.clone());
    }

    fn update_channel_create(&self, value: &ChannelCreateEvent) {
        let mut guild = self.insert_guild(value.channel.base.guild_id);
        guild.channels.insert((&value.channel).into());
    }

    fn update_channel_delete(&self, value: &ChannelDeleteEvent) {
        if let Some(mut guild) = self.guilds.get_mut(&value.channel.base.guild_id) {
            guild.channels.remove(&value.channel.id);

            // make sure to remove associated threads
            // they don't get their own delete event in this case
            guild.remove_associated_threads(value.channel.id);
        }
    }

    fn update_channel_update(&self, value: &ChannelUpdateEvent) {
        let mut guild = self.insert_guild(value.channel.base.guild_id);
        guild.channels.insert((&value.channel).into());
    }

    fn update_guild_create(&self, value: &GuildCreateEvent) {
        // only available guilds will send a create
        let Guild {
            channels, threads, ..
        } = &value.guild;

        let mut guild = CachedGuild {
            channels: channels.iter().map(CachedChannel::from).collect(),
            threads: threads.iter().map(CachedThread::from).collect(),
            threads_in: HashMap::new(),
        };

        // associate existing threads
        for thread in &guild.threads {
            guild
                .threads_in
                .entry(thread.parent_id)
                .or_default()
                .insert(thread.id);
        }

        self.guilds.insert(value.guild.id, guild);
    }

    fn update_guild_delete(&self, value: &GuildDeleteEvent) {
        self.guilds.remove(&value.guild.id);
    }

    fn update_user_update(&self, event: &UserUpdateEvent) {
        self.set_current_user(event.current_user.clone());
    }

    fn update_thread_create(&self, value: &ThreadCreateEvent) {
        // reasonably assume that only active threads can be created
        let mut guild = self.insert_guild(value.thread.base.guild_id);
        guild.add_thread((&value.thread).into());
    }

    fn update_thread_delete(&self, value: &ThreadDeleteEvent) {
        if let Some(mut guild) = self.guilds.get_mut(&value.thread.guild_id) {
            guild.remove_thread(value.thread.parent_id, value.thread.id);
        }
    }

    fn update_thread_update(&self, value: &ThreadUpdateEvent) {
        let thread = CachedThread::from(&value.thread);
        let mut guild = self.insert_guild(thread.guild_id);

        // we only track active threads so remove archived ones
        if value.thread.thread_metadata.archived() {
            guild.remove_thread(thread.parent_id, thread.id);
        } else {
            guild.add_thread(thread);
        }
    }

    fn update_thread_list_sync(&self, value: &ThreadListSyncEvent) {
        let mut guild = self.insert_guild(value.guild_id);

        if let Some(parents) = &value.channel_ids {
            for &channel_id in parents {
                guild.remove_associated_threads(channel_id);
            }
        }

        for thread in &value.threads {
            guild.add_thread(thread.into());
        }
    }

    // this event is received even without the required intents when the current
    // user is added to or removed from a thread.
    fn update_thread_members_update(&self, value: &ThreadMembersUpdateEvent) {
        let Some(user_id) = self.current_user_id() else {
            log::warn!("Current User is unset.");
            return;
        };

        if value.removed_member_ids.contains(&user_id) {
            self.remove_thread_if_private(value.guild_id, value.id);
        }
    }
}
