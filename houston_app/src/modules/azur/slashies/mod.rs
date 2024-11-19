use super::buttons;
use crate::prelude::*;
use crate::slashies::{command_group, create_reply};

mod autocomplete;
mod choices;
mod find;

use choices::*;

command_group!(
    /// Information about mobile game Azur Lane.
    pub azur (
        interaction_context = "Guild | BotDm | PrivateChannel",
    ),
    "ship", "search_ship",
    "equip", "search_equip",
    "augment", "search_augment",
    "reload_time",
);

/// Shows information about a ship.
#[poise::command(slash_command)]
async fn ship(
    ctx: HContext<'_>,
    #[description = "The ship's name. This supports auto completion."]
    #[autocomplete = "autocomplete::ship_name"]
    name: String,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> HResult {
    let data = ctx.data_ref();
    let ship = find::ship(data, &name)?;

    let view = buttons::ship::View::new(ship.group_id);
    ctx.send(view.modify_with_ship(data, ship, None).ephemeral(ephemeral.into_ephemeral())).await?;
    Ok(())
}

/// Searches for ships.
#[poise::command(slash_command, rename = "search-ship")]
async fn search_ship(
    ctx: HContext<'_>,
    #[description = "A name to search for."]
    name: Option<String>,
    #[description = "The faction to select."]
    faction: Option<EFaction>,
    #[description = "The hull type to select."]
    #[rename = "hull-type"]
    hull_type: Option<EHullType>,
    #[description = "The rarity to select."]
    rarity: Option<EShipRarity>,
    #[description = "Whether the ships have a unique augment."]
    #[rename = "has-augment"]
    has_augment: Option<bool>,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> HResult {
    use buttons::search_ship::*;

    let data = ctx.data_ref();

    let filter = Filter {
        name,
        faction: faction.map(EFaction::convert),
        hull_type: hull_type.map(EHullType::convert),
        rarity: rarity.map(EShipRarity::convert),
        has_augment
    };

    let view = View::new(filter);
    ctx.send(view.modify(data).ephemeral(ephemeral.into_ephemeral())).await?;

    Ok(())
}

/// Shows information about equipment.
#[poise::command(slash_command)]
async fn equip(
    ctx: HContext<'_>,
    #[description = "The equipment name. This supports auto completion."]
    #[autocomplete = "autocomplete::equip_name"]
    name: String,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> HResult {
    let data = ctx.data_ref();
    let equip = find::equip(data, &name)?;

    let view = buttons::equip::View::new(equip.equip_id);
    ctx.send(view.modify_with_equip(equip).ephemeral(ephemeral.into_ephemeral())).await?;
    Ok(())
}

/// Searches for equipment.
#[poise::command(slash_command, rename = "search-equip")]
async fn search_equip(
    ctx: HContext<'_>,
    #[description = "A name to search for."]
    name: Option<String>,
    #[description = "The faction to select."]
    faction: Option<EFaction>,
    #[description = "The kind to select."]
    kind: Option<EEquipKind>,
    #[description = "The rarity to select."]
    rarity: Option<EEquipRarity>,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> HResult {
    use buttons::search_equip::*;

    let data = ctx.data_ref();

    let filter = Filter {
        name,
        faction: faction.map(EFaction::convert),
        kind: kind.map(EEquipKind::convert),
        rarity: rarity.map(EEquipRarity::convert),
    };

    let view = View::new(filter);
    ctx.send(view.modify(data).ephemeral(ephemeral.into_ephemeral())).await?;

    Ok(())
}

/// Shows information about an augment module.
#[poise::command(slash_command)]
async fn augment(
    ctx: HContext<'_>,
    #[description = "The equipment name. This supports auto completion."]
    #[autocomplete = "autocomplete::augment_name"]
    name: String,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> HResult {
    let data = ctx.data_ref();
    let augment = find::augment(data, &name)?;

    let view = buttons::augment::View::new(augment.augment_id);
    ctx.send(view.modify_with_augment(data, augment).ephemeral(ephemeral.into_ephemeral())).await?;
    Ok(())
}

/// Searches for augment modules.
#[poise::command(slash_command, rename = "search-augment")]
async fn search_augment(
    ctx: HContext<'_>,
    #[description = "A name to search for."]
    name: Option<String>,
    #[description = "The allowed hull type."]
    #[rename = "hull-type"]
    hull_type: Option<EHullType>,
    #[description = "The rarity to select."]
    rarity: Option<EAugmentRarity>,
    #[description = "The name of the ship it is uniquely for."]
    #[autocomplete = "autocomplete::ship_name"]
    #[rename = "for-ship"]
    for_ship: Option<String>,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> HResult {
    use buttons::search_augment::*;

    let data = ctx.data_ref();

    let unique_ship_id = match for_ship {
        Some(for_ship) => Some(find::ship(data, &for_ship)?.group_id),
        None => None,
    };

    let filter = Filter {
        name,
        hull_type: hull_type.map(EHullType::convert),
        rarity: rarity.map(EAugmentRarity::convert),
        unique_ship_id,
    };

    let view = View::new(filter);
    ctx.send(view.modify(data).ephemeral(ephemeral.into_ephemeral())).await?;

    Ok(())
}

/// Calculates the actual reload time for a weapon.
#[poise::command(slash_command, rename = "reload-time")]
async fn reload_time(
    ctx: HContext<'_>,
    #[description = "The ship's RLD stat."]
    #[min = 1] #[max = 999]
    rld: f64,
    #[description = "The weapon's base FR in seconds."]
    #[min = 0.0] #[max = 60.0]
    #[rename = "weapon-fr"]
    weapon_reload: f64,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> HResult {
    let reload_time = (200.0 / (100.0 + rld)).sqrt() * weapon_reload;

    let description = format!(
        "-# **Base Weapon FR:** {weapon_reload:.2}s \u{2E31} **`RLD:`**`{rld: >4}`\n\
         **Final FR:** {reload_time:.2}s"
    );

    let embed = CreateEmbed::new()
        .color(ctx.data_ref().config().embed_color)
        .description(description);

    ctx.send(create_reply(ephemeral).embed(embed)).await?;
    Ok(())
}
