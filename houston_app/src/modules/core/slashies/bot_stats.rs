use crate::fmt::discord::{get_unique_username, TimeMentionable};
use crate::helper::time::get_startup_time;
use crate::prelude::*;

/// Shows information about the current bot session.
#[poise::command(
    slash_command,
    rename = "bot-stats",
    interaction_context = "BotDm",
)]
pub async fn bot_stats(
    ctx: HContext<'_>,
) -> HResult {
    let data = ctx.data_ref();

    let startup = get_startup_time().short_date_time();
    let version = env!("CARGO_PKG_VERSION");
    let git_hash = option_env!("GIT_HASH").unwrap_or("<unknown>");

    let current_user = data.current_user()?;
    let author = get_unique_username(current_user);
    let author = CreateEmbedAuthor::new(author).icon_url(current_user.face());

    let description = format!(
        "**Started:** {startup}\n\
         **Version:** `{version}`\n\
         **Git Rev:** `{git_hash}`",
    );

    let footer = CreateEmbedFooter::new("Houston Tools");

    let embed = CreateEmbed::new()
        .author(author)
        .description(description)
        .footer(footer)
        .color(ctx.data_ref().config().embed_color);

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}
