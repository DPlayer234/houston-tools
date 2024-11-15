use anyhow::Context;

use crate::prelude::*;
use crate::slashies::GUILD_INSTALL_ONLY;

/// View the server shop.
#[poise::command(
    slash_command,
    guild_only,
    custom_data = GUILD_INSTALL_ONLY,
)]
pub async fn shop(
    ctx: HContext<'_>,
) -> HResult {
    use crate::modules::perks::buttons::shop::View;

    let guild_id = ctx.guild_id().context("must be used in guild")?;

    ctx.defer_ephemeral().await?;

    let reply = View::new().create_reply(ctx.serenity_context(), guild_id, ctx.author().id).await?;
    ctx.send(reply.ephemeral(true)).await?;
    Ok(())
}