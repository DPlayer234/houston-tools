use std::borrow::Cow;

use anyhow::Context;

use crate::prelude::*;

use super::StarboardEntry;

crate::slashies::command_group!(
    /// Access starboard info.
    pub starboard (guild_only),
    "top", "top_posts",
);

/// Shows a board's top users.
#[poise::command(slash_command)]
async fn top(
    ctx: HContext<'_>,
    #[description = "What board to look for."]
    #[autocomplete = "autocomplete_board"]
    board: String,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> HResult {
    use super::buttons::top::View;

    let board = find_board(&ctx, &board)?;
    let view = View::new(board.channel);

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
    board: String,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> HResult {
    use super::buttons::top_posts::View;

    let board = find_board(&ctx, &board)?;
    let view = View::new(board.channel);

    ctx.defer_as(ephemeral).await?;
    ctx.send(view.create_reply(ctx.data_ref()).await?).await?;

    Ok(())
}

fn find_board<'a>(ctx: &HContext<'a>, board: &str) -> anyhow::Result<&'a StarboardEntry> {
    let guild_id = ctx.guild_id()
        .context("command only available in guilds")?;

    let channel_id = board.parse::<ChannelId>().ok()
        .ok_or(HArgError("Invalid board."))?;

    let board = ctx.data_ref()
        .config()
        .starboard
        .iter()
        .find(|b| b.channel == channel_id && b.guild == guild_id)
        .ok_or(HArgError("Unknown Starboard."))?;

    Ok(board)
}

async fn autocomplete_board<'a>(
    ctx: HContext<'a>,
    _partial: &'a str,
) -> CreateAutocompleteResponse<'a> {
    let choices: Vec<_> = ctx
        .data_ref()
        .config()
        .starboard
        .iter()
        .filter(|b| Some(b.guild) == ctx.guild_id())
        .map(|b| AutocompleteChoice::new(
            b.name.as_str(),
            Cow::Owned(b.channel.to_string()),
        ))
        .collect();

    CreateAutocompleteResponse::new()
        .set_choices(choices)
}
