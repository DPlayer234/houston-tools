use chrono::Utc;
use utils::text::write_str::*;

use crate::buttons::ButtonValue as _;
use crate::modules::core::buttons::Delete;
use crate::slashies::prelude::*;

/// "Snipes" and reveals the most recent, deleted message in this channel.
#[chat_command(contexts = "Guild", integration_types = "Guild")]
pub async fn snipe(ctx: Context<'_>) -> Result {
    let data = ctx.data_ref();
    let guild_id = ctx.require_guild_id()?;

    let snipe = data
        .config()
        .snipe
        .get(&guild_id)
        .ok_or(HArgError::new_const(
            "Message sniping is not enabled for this server.",
        ))?;

    let message = {
        let channel_id = ctx.channel_id();
        let min_timestamp = Utc::now()
            .checked_sub_signed(snipe.max_age)
            .context("max_age lands before beginning of time")?;

        // can't hold this lock across awaits, and it should be released asap anyways
        let mut state = snipe.state.write().expect("should not be poisoned");
        state.take_last(move |m| {
            m.deleted && m.channel_id == channel_id && *m.timestamp >= min_timestamp
        })
    };

    if let Some(message) = message {
        let author =
            CreateEmbedAuthor::new(message.author.display_name).icon_url(message.author.avatar_url);

        let mut embed = CreateEmbed::new()
            .author(author)
            .description(message.content)
            .timestamp(message.timestamp)
            .color(data.config().embed_color);

        if !message.attachments.is_empty() {
            let mut value = String::new();
            for attachment in &message.attachments {
                writeln_str!(value, "- [{}]({})", attachment.filename, attachment.url);
            }

            embed = embed.field("Attachments", value, false);
        }

        let button = CreateButton::new(Delete.to_custom_id())
            .style(ButtonStyle::Danger)
            .label("Delete");

        let row = [button];
        let row = CreateActionRow::buttons(&row);
        let row = &[row];

        let reply = CreateReply::new().embed(embed).components(row);
        ctx.send(reply).await?;
    } else {
        let content = format!(
            "No messages to snipe in {} right now.",
            ctx.channel_id().mention()
        );

        let embed = CreateEmbed::new()
            .description(content)
            .color(ERROR_EMBED_COLOR);

        let reply = CreateReply::new().embed(embed).ephemeral(true);
        ctx.send(reply).await?;
    }

    Ok(())
}
