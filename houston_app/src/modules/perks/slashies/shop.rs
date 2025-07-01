use crate::helper::discord::components::components;
use crate::slashies::prelude::*;

/// View the server shop.
#[chat_command(contexts = "Guild", integration_types = "Guild")]
pub async fn shop(ctx: Context<'_>) -> Result {
    use crate::modules::perks::buttons::shop::View;

    let guild_id = ctx.require_guild_id()?;

    // CMBK: interaction edits can't currently set flags in serenity, so to "defer"
    // and still allow components v2, we need to set the flag in the initial
    // message. but setting the flag with a defer doesn't work. discord moment. come
    // back when it works via edit.
    let reply = CreateReply::new()
        .ephemeral(true)
        .components_v2(components![CreateTextDisplay::new("Please wait...")]);

    let msg = ctx.send(reply).await?;

    let reply = View::new()
        .create_reply(ctx.serenity, guild_id, ctx.user().id)
        .await?;

    msg.edit(reply.into()).await?;
    Ok(())
}
