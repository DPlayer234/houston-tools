use rand::prelude::*;

use crate::slashies::prelude::*;

/// Flips a coin.
#[chat_command(
    contexts = "Guild | BotDm | PrivateChannel",
)]
pub async fn coin(
    ctx: Context<'_>,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> Result {
    const EDGE_TOSS_CHANCE: f64 = 1f64 / 6000f64;
    let content = {
        let mut rng = thread_rng();
        if rng.gen_bool(EDGE_TOSS_CHANCE) {
            "## Edge?!"
        } else if rng.gen_bool(0.5f64) {
            "### Heads!"
        } else {
            "### Tails!"
        }
    };

    let embed = CreateEmbed::new()
        .description(content)
        .color(ctx.data_ref().config().embed_color);

    ctx.send(create_reply(ephemeral).embed(embed)).await?;
    Ok(())
}
