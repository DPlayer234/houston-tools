use rand::prelude::*;

use crate::helper::discord::components::components_array;
use crate::slashies::prelude::*;

/// Flips a coin.
#[chat_command(
    contexts = "Guild | BotDm | PrivateChannel",
    integration_types = "Guild | User"
)]
pub async fn coin(
    ctx: Context<'_>,
    /// Whether to show the response only to yourself.
    ephemeral: Option<bool>,
) -> Result {
    const EDGE_TOSS_CHANCE: f64 = 1f64 / 6000f64;
    let content = {
        let mut rng = rand::rng();
        if rng.random_bool(EDGE_TOSS_CHANCE) {
            "## Edge?!"
        } else if rng.random_bool(0.5f64) {
            "### Heads!"
        } else {
            "### Tails!"
        }
    };

    let components = components_array![content];
    let components = components_array![
        CreateContainer::new(&components).accent_color(ctx.data_ref().config().embed_color),
    ];

    ctx.send(create_reply(ephemeral).components_v2(&components))
        .await?;
    Ok(())
}
