use anyhow::Context;

use super::BoardId;
use crate::prelude::*;

crate::slashies::command_group!(
    /// Access starboard info.
    pub starboard (
        guild_only,
        install_context = "Guild",
        interaction_context = "Guild",
    ),
    "top", "top_posts",
);

/// Shows a board's top users.
#[poise::command(slash_command)]
async fn top(
    ctx: HContext<'_>,
    #[description = "What board to look for."]
    #[autocomplete = "autocomplete_board"]
    board: u64,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> HResult {
    use super::buttons::top::View;

    let (guild, board) = find_board(&ctx, board)?;
    let view = View::new(guild, board);

    ctx.defer_as(ephemeral).await?;
    ctx.send(view.create_reply(ctx.data_ref()).await?).await?;

    Ok(())
}

/// Shows the most-reacted posts in a board.
#[poise::command(slash_command, rename = "top-posts")]
async fn top_posts(
    ctx: HContext<'_>,
    #[description = "What board to look for."]
    #[autocomplete = "autocomplete_board"]
    board: u64,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> HResult {
    use super::buttons::top_posts::View;

    let (guild, board) = find_board(&ctx, board)?;
    let view = View::new(guild, board);

    ctx.defer_as(ephemeral).await?;
    ctx.send(view.create_reply(ctx.data_ref()).await?).await?;

    Ok(())
}

fn find_board(ctx: &HContext<'_>, board: u64) -> anyhow::Result<(GuildId, BoardId)> {
    let guild_id = ctx.guild_id()
        .context("command only available in guilds")?;

    #[allow(clippy::cast_possible_wrap)]
    let board = BoardId::new(board as i64);
    _ = ctx
        .data_ref()
        .config()
        .starboard
        .get(&guild_id)
        .ok_or(HArgError::new_const("Starboard is not enabled for this server."))?
        .boards
        .get(&board)
        .ok_or(HArgError::new_const("Unknown Starboard."))?;

    Ok((guild_id, board))
}

async fn autocomplete_board<'a>(
    ctx: HContext<'a>,
    partial: &'a str,
) -> CreateAutocompleteResponse<'a> {
    if let Some(guild_id) = ctx.guild_id() {
        let choices: Vec<_> = ctx
            .data_ref()
            .config()
            .starboard
            // get the config for this guild and flatten into the board iter
            .get(&guild_id)
            .into_iter()
            .flat_map(|g| &g.boards)
            // filter to ones whose name contains the input
            // if the input is empty, that's all of them
            .filter(|(_, board)| board.name.contains(partial))
            // map it to an autocomplete choice with the board id as the value
            .map(|(id, board)| AutocompleteChoice::new(
                board.name.as_str(),
                #[allow(clippy::cast_sign_loss)]
                AutocompleteValue::Integer(id.get() as u64),
            ))
            .collect();

        CreateAutocompleteResponse::new()
            .set_choices(choices)
    } else {
        CreateAutocompleteResponse::new()
    }
}
