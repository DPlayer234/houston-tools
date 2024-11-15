use anyhow::Context;

use crate::prelude::*;
use crate::slashies::GUILD_INSTALL_ONLY;

crate::slashies::command_group!(
    /// Access starboard info.
    pub starboard (guild_only),
    "top", "top_posts",
);

/// Shows a board's top users.
#[poise::command(
    slash_command,
    custom_data = GUILD_INSTALL_ONLY,
)]
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
#[poise::command(
    slash_command,
    rename = "top-posts",
    custom_data = GUILD_INSTALL_ONLY,
)]
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

fn find_board(ctx: &HContext<'_>, board: u64) -> anyhow::Result<(GuildId, ChannelId)> {
    let guild_id = ctx.guild_id()
        .context("command only available in guilds")?;

    let board = usize::try_from(board).ok()
        .ok_or(HArgError::new_const("Invalid Starboard."))?;

    let board = ctx
        .data_ref()
        .config()
        .starboard
        .get(&guild_id)
        .ok_or(HArgError::new_const("Starboard is not enabled for this server."))?
        .boards
        .get(board)
        .ok_or(HArgError::new_const("Unknown Starboard."))?;

    Ok((guild_id, board.channel))
}

async fn autocomplete_board<'a>(
    ctx: HContext<'a>,
    _partial: &'a str,
) -> CreateAutocompleteResponse<'a> {
    if let Some(guild_id) = ctx.guild_id() {
        let choices: Vec<_> = ctx
            .data_ref()
            .config()
            .starboard
            .get(&guild_id)
            .into_iter()
            .flat_map(|g| &g.boards)
            .enumerate()
            .map(|(index, board)| AutocompleteChoice::new(
                board.name.as_str(),
                AutocompleteValue::Integer(index as u64),
            ))
            .collect();

        CreateAutocompleteResponse::new()
            .set_choices(choices)
    } else {
        CreateAutocompleteResponse::new()
    }
}
