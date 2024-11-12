use anyhow::Context;

use crate::prelude::*;

/// Obtain and check perks.
#[poise::command(slash_command, rename = "perk-store", guild_only)]
pub async fn perk_store(
    ctx: HContext<'_>,
) -> HResult {
    use crate::modules::perks::buttons::perk_store::View;

    let guild_id = ctx.guild_id().context("must be used in guild")?;

    ctx.defer_ephemeral().await?;

    let reply = View::new().create_reply(ctx.serenity_context(), guild_id, ctx.author().id).await?;
    ctx.send(reply.ephemeral(true)).await?;
    Ok(())
}
