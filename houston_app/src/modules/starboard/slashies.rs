use anyhow::Context;

use crate::prelude::*;

crate::slashies::command_group!(
    /// Access starboard info.
    pub starboard (guild_only),
    "leaderboard",
);

/// Shows a starboard's leaderboard.
#[poise::command(slash_command, guild_only)]
async fn leaderboard(
    ctx: HContext<'_>,
    #[description = "Which board to look for, identified by emoji."]
    #[autocomplete = "autocomplete_board"]
    board: String,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> HResult {
    use super::buttons::leaderboard::View;

    let guild_id = ctx.guild_id()
        .context("command only available in guilds")?;

    let ephemeral = ephemeral.unwrap_or(true);
    let board = ctx.data_ref()
        .config()
        .starboard
        .iter()
        .find(|b| b.emoji.name() == board && b.guild == guild_id)
        .ok_or(HArgError("Unknown Starboard."))?;

    let view = View {
        board: board.channel,
        page: 0,
        ephemeral,
    };

    ctx.defer_as(ephemeral).await?;
    ctx.send(view.create_reply(ctx.data_ref()).await?).await?;

    Ok(())
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
        .map(|b| AutocompleteChoice::from(b.emoji.name()))
        .collect();

    CreateAutocompleteResponse::new()
        .set_choices(choices)
}
