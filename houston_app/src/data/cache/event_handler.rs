use std::collections::HashMap;

use serenity::futures::future::{BoxFuture, always_ready};
use serenity::gateway::client::{Context, RawEventHandler};

use super::model::*;
use super::{Cache, CachedGuild};
use crate::prelude::*;

/// Manual `async_trait` impl so avoid allocation.
impl RawEventHandler for Cache {
    fn raw_event<'s: 'f, 'e: 'f, 'f>(&'s self, _ctx: Context, ev: &'e Event) -> BoxFuture<'f, ()> {
        self.update_event(ev);
        Box::pin(always_ready(|| {}))
    }
}

/// Internal methods to update the cache based on received events.
impl Cache {
    /// Removes a thread from the cache, if it is a private thread.
    fn remove_thread_if_private(&self, guild_id: GuildId, thread_id: ChannelId) {
        if let Some(mut guild) = self.guilds.get_mut(&guild_id) {
            if let Some(thread) = guild.threads.get(&thread_id) {
                if thread.kind == ChannelType::PrivateThread {
                    guild.threads.remove(&thread_id);
                }
            }
        }
    }

    /// Updates the cache with an event.
    fn update_event(&self, event: &Event) {
        match event {
            Event::Ready(event) => self.update_ready(event),
            Event::ChannelCreate(event) => self.update_channel_create(event),
            Event::ChannelDelete(event) => self.update_channel_delete(event),
            Event::ChannelUpdate(event) => self.update_channel_update(event),
            Event::GuildCreate(event) => self.update_guild_create(event),
            Event::GuildDelete(event) => self.update_guild_delete(event),
            Event::UserUpdate(event) => self.update_user_update(event),
            Event::ThreadCreate(event) => self.update_thread_create(event),
            Event::ThreadUpdate(event) => self.update_thread_update(event),
            Event::ThreadDelete(event) => self.update_thread_delete(event),
            Event::ThreadListSync(event) => self.update_thread_list_sync(event),
            Event::ThreadMembersUpdate(event) => self.update_thread_members_update(event),
            _ => {},
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
        let mut guild = self.insert_guild(value.channel.guild_id);
        guild.channels.insert((&value.channel).into());
    }

    fn update_channel_delete(&self, value: &ChannelDeleteEvent) {
        if let Some(mut guild) = self.guilds.get_mut(&value.channel.guild_id) {
            guild.channels.remove(&value.channel.id);

            // make sure to remove associated threads
            // they don't get their own delete event in this case
            guild.remove_associated_threads(value.channel.id);
        }
    }

    fn update_channel_update(&self, value: &ChannelUpdateEvent) {
        let mut guild = self.insert_guild(value.channel.guild_id);
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
        let mut guild = self.insert_guild(value.thread.guild_id);
        guild.add_thread((&value.thread).into());
    }

    fn update_thread_delete(&self, value: &ThreadDeleteEvent) {
        if let Some(mut guild) = self.guilds.get_mut(&value.thread.guild_id) {
            guild.remove_thread(value.thread.parent_id, value.thread.id);
        }
    }

    fn update_thread_update(&self, value: &ThreadUpdateEvent) {
        let Some(metadata) = &value.thread.thread_metadata else {
            let id = value.thread.id;
            log::warn!("Thread Update for {id} didn't have metadata.");
            return;
        };

        let thread = CachedThread::from(&value.thread);
        let mut guild = self.insert_guild(thread.guild_id);

        // we only track active threads so remove archived ones
        if metadata.archived() {
            guild.remove_thread(thread.parent_id, thread.id);
        } else {
            guild.add_thread(thread);
        }
    }

    fn update_thread_list_sync(&self, value: &ThreadListSyncEvent) {
        let mut guild = self.guilds.entry(value.guild_id).or_default();

        if let Some(parents) = &value.channel_ids {
            for &channel_id in parents {
                guild.remove_associated_threads(channel_id);
            }
        }

        for thread in &value.threads {
            guild.add_thread(thread.into());
        }
    }

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
