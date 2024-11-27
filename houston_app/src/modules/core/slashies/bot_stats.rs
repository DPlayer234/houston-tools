use std::time::Instant;

use utils::text::write_str::*;

use crate::fmt::discord::{get_unique_username, TimeMentionable};
use crate::helper::time::get_startup_time;
use crate::slashies::prelude::*;

/// Shows information about the current bot session.
#[chat_command(
    name = "bot-stats",
    contexts = "BotDm",
    integration_types = "Guild | User",
)]
pub async fn bot_stats(
    ctx: Context<'_>,
) -> Result {
    let data = ctx.data_ref();

    let startup = get_startup_time().short_date_time();
    let version = env!("CARGO_PKG_VERSION");
    let git_hash = option_env!("GIT_HASH").unwrap_or("<unknown>");

    let current_user = data.current_user()?;
    let author = get_unique_username(current_user);
    let author_icon = current_user.face();

    // both of these only borrow the data so cloning the resulting embed is cheap
    let author = CreateEmbedAuthor::new(&*author).icon_url(&author_icon);
    let footer = CreateEmbedFooter::new("Houston Tools");

    let base_embed = CreateEmbed::new()
        .author(author)
        .footer(footer)
        .color(data.config().embed_color);

    // 128 bytes is enough for the entire description
    // the code here is slightly weird so we can reuse the buffer
    let mut description = String::with_capacity(128);
    write_str!(
        description,
        "**Started:** {startup}\n\
         **Version:** `{version}`\n\
         **Git Rev:** `{git_hash}`\n\
         **Ping:** <wait>"
    );

    let embed = base_embed.clone().description(&description);
    let now = Instant::now();
    let reply = ctx.send(CreateReply::new().embed(embed)).await?;

    let elapsed = now.elapsed().as_millis();

    description.clear();
    write_str!(
        description,
        "**Started:** {startup}\n\
         **Version:** `{version}`\n\
         **Git Rev:** `{git_hash}`\n\
         **Ping:** {elapsed} ms"
    );

    let embed = base_embed.description(description);
    reply.edit(EditReply::new().embed(embed)).await?;
    Ok(())
}
