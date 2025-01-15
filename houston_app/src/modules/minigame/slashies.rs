use crate::slashies::prelude::*;

/// Play games.
#[chat_command(
    name = "minigame",
    contexts = "Guild | PrivateChannel",
    integration_types = "Guild | User"
)]
pub async fn root(_ctx: Context<'_>) -> Result {
    anyhow::bail!("never intended to be called")
}

fn check_user(user: &User) -> Result {
    anyhow::ensure!(
        !user.bot() && !user.system(),
        HArgError::new_const("You can't invite bots to play a game.")
    );
    Ok(())
}

/// Play tic-tac-toe with someone else.
#[sub_command(name = "tic-tac-toe")]
pub async fn tic_tac_toe(
    ctx: Context<'_>,
    /// The user to play against.
    opponent: &User,
) -> Result {
    use crate::modules::minigame::buttons::tic_tac_toe::View;

    check_user(opponent)?;
    let players = [ctx.user().id, opponent.id];
    let reply = View::new(players).create_next_reply(ctx.data_ref());
    ctx.send(reply).await?;
    Ok(())
}
