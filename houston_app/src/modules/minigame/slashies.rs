use crate::slashies::prelude::*;

/// Play games.
#[chat_command(
    contexts = "Guild | PrivateChannel",
    integration_types = "Guild | User"
)]
pub mod minigame {
    /// Play tic-tac-toe with someone else.
    #[sub_command(name = "tic-tac-toe")]
    async fn tic_tac_toe(
        ctx: Context<'_>,
        /// The user to play against.
        opponent: &User,
    ) -> Result {
        use crate::modules::minigame::buttons::tic_tac_toe::View;

        check_user(&ctx, opponent)?;
        let players = [ctx.user().id, opponent.id];
        let reply = View::new(players).create_next_reply(ctx.data_ref());
        ctx.send(reply).await?;
        Ok(())
    }

    /// Play rock-paper-scissors with someone else.
    #[sub_command(name = "rock-paper-scissors")]
    async fn rock_paper_scissors(
        ctx: Context<'_>,
        /// The user to play against.
        opponent: &User,
    ) -> Result {
        use crate::modules::minigame::buttons::rock_paper_scissors::View;

        check_user(&ctx, opponent)?;
        let players = [ctx.user().id, opponent.id];
        let reply = View::new(players).create_next_reply(ctx.data_ref());
        ctx.send(reply).await?;
        Ok(())
    }
}

fn check_user(ctx: &Context<'_>, user: &User) -> Result {
    anyhow::ensure!(
        ctx.user().id != user.id,
        HArgError::new_const("Do you not have friends?")
    );
    anyhow::ensure!(
        !user.bot() && !user.system(),
        HArgError::new_const("You can't invite bots to play these games.")
    );
    Ok(())
}
