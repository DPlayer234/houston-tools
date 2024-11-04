use rand::prelude::*;

use crate::prelude::*;

/// Flips a coin.
#[poise::command(slash_command)]
pub async fn coin(
    ctx: HContext<'_>
) -> HResult {
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
        .color(DEFAULT_EMBED_COLOR);

    ctx.send(ctx.create_reply().embed(embed)).await?;
    Ok(())
}
