use super::prelude::*;
use crate::config::HBotConfig;
use crate::fmt::discord::MessageLinkExt as _;
use crate::helper::discord::is_user_message;
use crate::helper::is_unique_set;

pub mod config;

pub use config::Config;
use config::{MediaCheck, MediaReactChannel};

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &HBotConfig) -> bool {
        !config.media_react.is_empty()
    }

    fn intents(&self, _config: &HBotConfig) -> GatewayIntents {
        // `GUILDS` for cache, `GUILD_MESSAGES` to know what to react to, and
        // `MESSAGE_CONTENT` to check whether to react
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT
    }

    fn validate(&self, config: &HBotConfig) -> Result {
        for (channel, channel_config) in &config.media_react {
            let emojis = &channel_config.emojis;

            anyhow::ensure!(
                is_unique_set(emojis.iter().map(|e| &e.emoji)),
                "media react channel {channel} has duplicate emojis"
            );

            anyhow::ensure!(
                emojis.len() <= 20,
                "media react channel {channel} has more than 20 emojis"
            );
        }

        log::info!(
            "Media reacts are enabled: {} channel(s)",
            config.media_react.len()
        );
        Ok(())
    }

    fn event_handler(self) -> Option<Box<dyn EventHandler>> {
        Some(Box::new(self))
    }
}

super::impl_handler!(Module, |_, ctx| match _ {
    FullEvent::Message { new_message, .. } => message(ctx, new_message),
});

pub async fn message(ctx: &Context, new_message: &Message) {
    if let Err(why) = message_inner(ctx, new_message).await {
        let message_key = new_message.link().key();
        log::error!("Message handling failed for {message_key}: {why:?}");
    }
}

async fn message_inner(ctx: &Context, new_message: &Message) -> Result {
    // we only consider regular messages from users, not bots
    if !is_user_message(new_message) {
        return Ok(());
    }

    // grab the config for the current channel
    let entries = find_channel_config(ctx, new_message.guild_id, new_message.channel_id).await?;
    let Some(channel_config) = entries else {
        return Ok(());
    };

    let mut check = MediaChecker::new(new_message);
    for entry in &channel_config.emojis {
        // if there is an attachment or the content has media links, attach the emoji to
        // the message. nested message snapshots (forwards) are checked the same way
        if check.with(entry.condition) {
            new_message
                .react(&ctx.http, entry.emoji.as_emoji().clone())
                .await
                .context("could not add media reaction")?;
        }
    }

    Ok(())
}

async fn find_channel_config(
    ctx: &Context,
    guild_id: Option<GuildId>,
    channel_id: GenericChannelId,
) -> Result<Option<&MediaReactChannel>> {
    let data = ctx.data_ref::<HContextData>();

    // first, attempt to get the config for the exact channel id
    if let Some(entries) = data.config().media_react.get(&channel_id) {
        return Ok(Some(entries));
    }

    // the following code is only applicable to guild channels
    let Some(guild_id) = guild_id else {
        return Ok(None);
    };

    // second, try if this is a thread
    let thread = data
        .cache()
        .thread_channel(&ctx.http, guild_id, channel_id.expect_thread())
        .await?;

    // if it is a thread, grab the parent channel's configuration
    // filter it on whether threads are included
    let entries = thread
        .and_then(|t| data.config().media_react.get(&t.parent_id.widen()))
        .filter(|c| c.with_threads);

    Ok(entries)
}

/// Provides a way to check whether a message or its snapshots have media, given
/// the check condition, avoiding repeated content checks if possible but only
/// actually performing them if they are needed.
#[derive(Debug)]
struct MediaChecker<'a> {
    message: &'a Message,
    normal: Option<bool>,
    forward: Option<bool>,
}

impl<'a> MediaChecker<'a> {
    fn new(message: &'a Message) -> Self {
        Self {
            message,
            normal: None,
            forward: None,
        }
    }

    fn with(&mut self, condition: MediaCheck) -> bool {
        if self.message.message_snapshots.is_empty() {
            condition.normal.select(|| self.normal())
        } else {
            condition.forward.select(|| self.forward())
        }
    }

    fn normal(&mut self) -> bool {
        fn normal_content(m: &Message) -> bool {
            !m.attachments.is_empty() || has_media_content(&m.content)
        }

        *self
            .normal
            .get_or_insert_with(|| normal_content(self.message))
    }

    fn forward(&mut self) -> bool {
        fn forward_content(m: &MessageSnapshot) -> bool {
            !m.attachments.is_empty() || has_media_content(&m.content)
        }

        *self
            .forward
            .get_or_insert_with(|| self.message.message_snapshots.iter().any(forward_content))
    }
}

fn has_media_content(content: &str) -> bool {
    fn includes_media_link(content: &str, prefix: &str) -> bool {
        content
            .match_indices(prefix)
            .any(|(index, _)| is_media_link_match(content, prefix, index))
    }

    fn is_media_link_match(content: &str, prefix: &str, index: usize) -> bool {
        // if a '<' comes first, this is masked and we ignore it
        if content.as_bytes().get(index.wrapping_sub(1)) == Some(&b'<') {
            return false;
        }

        // cut out the link itself, without the schema
        let index = index + prefix.len();
        let Some(content) = content.get(index..) else {
            return false;
        };

        // ignore certain domains
        // cdn links would be `cdn.discord.com`, so those should be unaffected
        !content.starts_with("discord.com") && !content.starts_with("discord.gg")
    }

    includes_media_link(content, "http://") || includes_media_link(content, "https://")
}

#[cfg(test)]
mod tests {
    use super::has_media_content;

    #[test]
    fn has_media() {
        assert!(has_media_content(
            "look here: https://cdn.discordapp.com/attachments/111/222/333.png"
        ));
        assert!(has_media_content(
            "oh my god how cute https://imgur.com/gallery/IpNHG9c !!"
        ));
        assert!(has_media_content("http://example.com/image"))
    }

    #[test]
    fn has_no_media() {
        assert!(!has_media_content(
            "look here: <https://cdn.discordapp.com/attachments/111/222/333.png>"
        ));
        assert!(!has_media_content(
            "oh my god how cute <https://imgur.com/gallery/IpNHG9c> !!"
        ));
        assert!(!has_media_content("<http://example.com/image>"));
        assert!(!has_media_content(
            "https://discord.com/channels/480539182201176065/541068693837316106/1306253817238523935"
        ));
        assert!(!has_media_content("https://discord.gg/invite/abcdef"));
    }
}
