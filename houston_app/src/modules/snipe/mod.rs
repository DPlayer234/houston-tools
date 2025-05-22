use super::prelude::*;
use crate::fmt::discord::MessageLink;
use crate::helper::discord::is_user_message;

pub mod config;
mod slashies;
pub mod state;

pub use config::Config;

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &HBotConfig) -> bool {
        !config.snipe.is_empty()
    }

    fn intents(&self, _config: &HBotConfig) -> GatewayIntents {
        // `GUILD_MESSAGES` and `MESSAGE_CONTENT` so messages can be snapshotted
        GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT
    }

    fn commands(&self, _config: &HBotConfig) -> impl IntoIterator<Item = Command> {
        [slashies::snipe()]
    }

    fn validate(&self, config: &HBotConfig) -> Result {
        log::info!("Snipe is enabled.");

        for (guild, snipe) in &config.snipe {
            if snipe.max_cache_size.get() >= usize::from(u16::MAX) {
                log::warn!(
                    "`snipe.{guild}.max_cache_size` is set to a large value ({}). \
                     This may cause increased memory usage and high CPU time on message deletion.",
                    snipe.max_cache_size
                );
            }
        }

        Ok(())
    }

    fn event_handler(self) -> Option<Box<dyn EventHandler>> {
        Some(Box::new(self))
    }
}

super::impl_handler!(Module, |_, ctx| match _ {
    FullEvent::Message { new_message, .. } => message(ctx, new_message, false),
    FullEvent::MessageUpdate { event, .. } => message(ctx, &event.message, true),
    FullEvent::MessageDelete {
        channel_id,
        deleted_message_id,
        guild_id,
        ..
    } => message_delete(ctx, *channel_id, *deleted_message_id, *guild_id),
});

async fn message(ctx: &Context, new_message: &Message, is_edit: bool) {
    let Some(guild_id) = new_message.guild_id else {
        return;
    };

    let message_link = MessageLink::from(new_message);

    if let Err(why) = message_inner(ctx, new_message, guild_id, is_edit) {
        log::error!("Message handling failed for {message_link:#}: {why:?}");
    }
}

fn message_inner(ctx: &Context, new_message: &Message, guild_id: GuildId, is_edit: bool) -> Result {
    // we only consider regular messages from users, not bots.
    // also ignore messages that have neither content nor attachments or ones that
    // have a lot of content. attachments aren't currently retained, but they are
    // noted. this essentially excludes sticker-only messages and polls.
    let valid = is_user_message(new_message)
        && (!new_message.content.is_empty() || !new_message.attachments.is_empty())
        && new_message.content.len() <= 2000;

    if !valid {
        return Ok(());
    }

    let data = ctx.data_ref::<HContextData>();
    let Some(snipe) = data.config().snipe.get(&guild_id) else {
        return Ok(());
    };

    let mut state = snipe.state.lock().expect("should not be poisoned");
    if is_edit {
        // if edited, try to look for the original and replace it
        // we don't bother to insert it as new since it may be old
        if let Some(known) = state.get_message_mut(new_message.id) {
            known.update(new_message);
        }
    } else {
        while state.messages.len() >= snipe.max_cache_size.get() {
            state.messages.pop_front();
        }

        let sniped = state::SnipedMessage::new(new_message);
        state.messages.push_back(sniped);
    }

    Ok(())
}

async fn message_delete(
    ctx: &Context,
    channel_id: GenericChannelId,
    message_id: MessageId,
    guild_id: Option<GuildId>,
) {
    let Some(guild_id) = guild_id else {
        return;
    };

    let message_link = MessageLink::new(guild_id, channel_id, message_id);

    if let Err(why) = message_delete_inner(ctx, guild_id, message_id) {
        log::error!("Message delete handling failed for {message_link:#}: {why:?}");
    }
}

fn message_delete_inner(ctx: &Context, guild_id: GuildId, message_id: MessageId) -> Result {
    let data = ctx.data_ref::<HContextData>();
    let Some(snipe) = data.config().snipe.get(&guild_id) else {
        return Ok(());
    };

    let mut state = snipe.state.lock().expect("should not be poisoned");
    if let Some(message) = state.get_message_mut(message_id) {
        message.deleted = true;
    }

    Ok(())
}
