use crate::internal::prelude::*;

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
    let ship = ctx.data().azur_lane.ship_by_name(&name);
    match ship {
        None => Err(ShipParseError)?,
        Some(ship) => {
            ctx.send(ctx.create_reply().content(format!("[ {} ]", ship.group_id))).await?;
            Ok(())
        }
    }
}

async fn autocomplete_name(ctx: HContext<'_>, partial: &str) -> Vec<String> {
    ctx.data().azur_lane
        .ships_by_prefix(partial)
        .map(|s| (*s.name).to_owned())
        .take(10)
        .collect()
}

#[derive(Debug, Clone)]
struct ShipParseError;

impl std::error::Error for ShipParseError {}

impl std::fmt::Display for ShipParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown ship name.")
    }
}
