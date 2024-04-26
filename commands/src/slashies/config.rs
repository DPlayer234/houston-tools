use crate::prelude::*;

/// Provides (temporary) configuration for this app.
#[poise::command(
    slash_command,
    subcommands("config_hide"),
    subcommand_required
)]
pub async fn config(_: HContext<'_>) -> HResult {
    Ok(())
}

/// Configures whether responses to your commands are hidden from other users.
#[poise::command(slash_command, rename = "hide")]
async fn config_hide(
    ctx: HContext<'_>,
    #[description = "Whether the responses are hidden. Starts at true."]
    hidden: Option<bool>
) -> HResult {
    let mut data = ctx.get_user_data();
    data.ephemeral = hidden.unwrap_or(!data.ephemeral);
    ctx.set_user_data(data.clone());

    let mut content = MessageBuilder::new();
    content.push("Your command usage is now ");

    content.push_bold(if data.ephemeral {
        "hidden"
    } else {
        "visible"
    });

    content.push(" to other users.");

    let embed = CreateEmbed::new()
        .description(content.build())
        .color(DEFAULT_EMBED_COLOR);

    ctx.send(ctx.create_ephemeral_reply().embed(embed)).await?;
    Ok(())
}