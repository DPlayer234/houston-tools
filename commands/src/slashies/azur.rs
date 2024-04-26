use crate::prelude::*;
use crate::buttons;

/// Information about mobile game Azur Lane.
#[poise::command(
    slash_command,
    subcommands("ship"),
    subcommand_required
)]
pub async fn azur(_: HContext<'_>) -> HResult {
    Ok(())
}

/// Shows information about a ship.
#[poise::command(slash_command)]
async fn ship(
    ctx: HContext<'_>,
    #[description = "The ship's name. This supports auto completion."]
    #[autocomplete = "autocomplete_name"]
    name: String
) -> HResult {
    let ship = ctx.data().azur_lane.ships_by_prefix(&name).next().ok_or(buttons::azur::ShipParseError)?;
    let view = buttons::azur::ship::ViewShip::new_with_ship_id(ship.group_id);
    ctx.send(view.modify_with_ship(ctx.create_reply(), ship, None)).await?;
    Ok(())
}

async fn autocomplete_name(ctx: HContext<'_>, partial: &str) -> Vec<String> {
    ctx.data().azur_lane
        .ships_by_prefix(partial)
        .map(|s| (*s.name).to_owned())
        .take(10)
        .collect()
}
