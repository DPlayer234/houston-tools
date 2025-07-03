use crate::helper::discord::components::temp_defer_as;
use crate::slashies::prelude::*;

/// View the server shop.
#[chat_command(contexts = "Guild", integration_types = "Guild")]
pub async fn shop(ctx: Context<'_>) -> Result {
    use crate::modules::perks::buttons::shop::View;

    let guild_id = ctx.require_guild_id()?;

    let msg = temp_defer_as(ctx, true).await?;

    let reply = View::new()
        .create_reply(ctx.serenity, guild_id, ctx.user().id)
        .await?;

    msg.edit(reply.into()).await?;
    Ok(())
}
