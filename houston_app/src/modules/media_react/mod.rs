use std::collections::HashMap;

use super::prelude::*;
use crate::config::HEmoji;
use crate::fmt::discord::MessageLink;
use crate::helper::is_unique_set;

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &super::config::HBotConfig) -> bool {
        !config.media_react.is_empty()
    }

    fn intents(&self, _config: &config::HBotConfig) -> GatewayIntents {
        GatewayIntents::MESSAGE_CONTENT
    }

    fn validate(&self, config: &config::HBotConfig) -> Result {
        for (channel, entry) in &config.media_react {
            anyhow::ensure!(
                is_unique_set(entry.emojis.iter()),
                "media react channel {channel} has duplicate emojis"
            );
        }

        log::info!(
            "Media reacts are enabled: {} channel(s)",
            config.media_react.len()
        );
        Ok(())
    }
}

pub type Config = HashMap<ChannelId, MediaChannelEntry>;

#[derive(Debug, serde::Deserialize)]
pub struct MediaChannelEntry {
    pub emojis: Vec<HEmoji>,
}

pub async fn message(ctx: Context, new_message: Message) {
    let message_link = MessageLink::from(&new_message);

    if let Err(why) = message_inner(ctx, new_message).await {
        log::error!("Message handling failed for {message_link:#}: {why:?}");
    }
}

async fn message_inner(ctx: Context, new_message: Message) -> Result {
    // we only consider regular messages from users, not bots
    let valid = is_normal_message(new_message.kind)
        && !new_message.author.bot()
        && !new_message.author.system();

    if !valid {
        return Ok(());
    }

    let data = ctx.data_ref::<HContextData>();

    // grab the config for the current channel
    let channel_config = data.config().media_react.get(&new_message.channel_id);

    let Some(channel_config) = channel_config else {
        return Ok(());
    };

    // if there is an attachment or the content has media links,
    // attach the emoji to the message
    // CMBK: check message snapshots when forwarding is fully implemented
    let has_media = !new_message.attachments.is_empty()
        || new_message
            .message_reference
            .as_ref()
            .is_some_and(|m| m.kind == MessageReferenceKind::Forward)
        || has_media_content(&new_message.content);

    if !has_media {
        return Ok(());
    }

    for emoji in &channel_config.emojis {
        new_message
            .react(&ctx.http, emoji.as_emoji().clone())
            .await
            .context("could not add media reaction")?;
    }

    Ok(())
}

fn is_normal_message(kind: MessageType) -> bool {
    matches!(kind, MessageType::Regular | MessageType::InlineReply)
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
