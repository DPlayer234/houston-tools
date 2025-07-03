use super::BoardId;
use crate::helper::contains_ignore_ascii_case;
use crate::slashies::prelude::*;

mod overview;

/// Access starboard info.
#[chat_command(contexts = "Guild", integration_types = "Guild")]
pub mod starboard {
    /// Shows a board's top users.
    #[sub_command]
    async fn top(
        ctx: Context<'_>,
        /// What board to look for.
        #[autocomplete = "autocomplete_board"]
        board: u64,
        /// Whether to show the response only to yourself.
        ephemeral: Option<bool>,
    ) -> Result {
        use super::buttons::top::View;

        let (guild, board) = find_board(ctx, board)?;
        let view = View::new(guild, board);

        ctx.defer_as(ephemeral).await?;
        ctx.send(view.create_reply(ctx.data_ref()).await?).await?;

        Ok(())
    }

    /// Shows the most-reacted posts in a board.
    #[sub_command(name = "top-posts")]
    async fn top_posts(
        ctx: Context<'_>,
        /// What board to look for.
        #[autocomplete = "autocomplete_board"]
        board: u64,
        /// Filter to posts by a specific user.
        #[name = "by-user"]
        by_user: Option<&User>,
        /// Whether to show the response only to yourself.
        ephemeral: Option<bool>,
    ) -> Result {
        use super::buttons::top_posts::View;

        let (guild, board) = find_board(ctx, board)?;
        let view = View::new(guild, board, by_user.map(|u| u.id));

        ctx.defer_as(ephemeral).await?;
        ctx.send(view.create_reply(ctx.data_ref()).await?).await?;

        Ok(())
    }

    #[sub_command]
    use overview::overview;
}

fn find_board(ctx: Context<'_>, board: u64) -> Result<(GuildId, BoardId)> {
    let guild_id = ctx.guild_id().context("command only available in guilds")?;

    let board = BoardId::new(board.cast_signed());
    _ = ctx
        .data_ref()
        .config()
        .starboard
        .get(&guild_id)
        .ok_or(HArgError::new_const(
            "Starboard is not enabled for this server.",
        ))?
        .boards
        .get(&board)
        .ok_or(HArgError::new_const("Unknown Starboard."))?;

    Ok((guild_id, board))
}

async fn autocomplete_board<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> CreateAutocompleteResponse<'a> {
    // get the config for this guild, return empty if none
    if let Some(guild_id) = ctx.guild_id()
        && let Some(guild_config) = ctx.data_ref().config().starboard.get(&guild_id)
    {
        let choices: Vec<_> = guild_config
            .boards
            .values()
            // filter to ones whose name contains the input
            // if the input is empty, that's all of them
            .filter(|board| contains_ignore_ascii_case(&board.name, partial))
            // map it to an autocomplete choice with the board id as the value
            .map(|board| {
                AutocompleteChoice::new(
                    board.name.as_str(),
                    AutocompleteValue::Integer(board.id.get().cast_unsigned()),
                )
            })
            .collect();

        CreateAutocompleteResponse::new().set_choices(choices)
    } else {
        CreateAutocompleteResponse::new()
    }
}
