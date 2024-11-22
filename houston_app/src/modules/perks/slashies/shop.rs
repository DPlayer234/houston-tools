use crate::slashies::prelude::*;

/// View the server shop.
#[chat_command(
    contexts = "Guild",
    integration_types = "Guild",
)]
pub async fn shop(
    ctx: Context<'_>,
) -> Result {
    use crate::modules::perks::buttons::shop::View;

    let guild_id = ctx.require_guild_id()?;

    ctx.defer_as(Ephemeral).await?;

    let reply = View::new().create_reply(ctx.serenity, guild_id, ctx.user().id).await?;
    ctx.send(reply.ephemeral(true)).await?;
    Ok(())
}
