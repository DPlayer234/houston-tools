use crate::prelude::*;
use crate::buttons;

mod autocomplete;
mod choices;
mod find;

use choices::*;

super::command_group!(
    /// Information about mobile game Azur Lane.
    pub azur,
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
    name: String
) -> HResult {
    let data = ctx.data();
    let ship = find::ship(&data, &name)?;

    let view = buttons::azur::ship::View::new(ship.group_id);
    ctx.send(view.modify_with_ship(&data, ctx.create_reply(), ship, None)).await?;
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
    has_augment: Option<bool>
) -> HResult {
    use crate::buttons::azur::search_ship::*;

    let data = ctx.data();

    let filter = Filter {
        name,
        faction: faction.map(EFaction::convert),
        hull_type: hull_type.map(EHullType::convert),
        rarity: rarity.map(EShipRarity::convert),
        has_augment
    };

    let view = View::new(filter);
    ctx.send(view.modify(&data, ctx.create_reply())).await?;

    Ok(())
}

/// Shows information about equipment.
#[poise::command(slash_command)]
async fn equip(
    ctx: HContext<'_>,
    #[description = "The equipment name. This supports auto completion."]
    #[autocomplete = "autocomplete::equip_name"]
    name: String
) -> HResult {
    let data = ctx.data();
    let equip = find::equip(&data, &name)?;

    let view = buttons::azur::equip::View::new(equip.equip_id);
    ctx.send(view.modify_with_equip(ctx.create_reply(), equip)).await?;
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
    rarity: Option<EEquipRarity>
) -> HResult {
    use crate::buttons::azur::search_equip::*;

    let data = ctx.data();

    let filter = Filter {
        name,
        faction: faction.map(EFaction::convert),
        kind: kind.map(EEquipKind::convert),
        rarity: rarity.map(EEquipRarity::convert),
    };

    let view = View::new(filter);
    ctx.send(view.modify(&data, ctx.create_reply())).await?;

    Ok(())
}

/// Shows information about an augment module.
#[poise::command(slash_command)]
async fn augment(
    ctx: HContext<'_>,
    #[description = "The equipment name. This supports auto completion."]
    #[autocomplete = "autocomplete::augment_name"]
    name: String
) -> HResult {
    let data = ctx.data();
    let augment = find::augment(&data, &name)?;

    let view = buttons::azur::augment::View::new(augment.augment_id);
    ctx.send(view.modify_with_augment(&data, ctx.create_reply(), augment)).await?;
    Ok(())
}

/// Searches for augment modules.
#[poise::command(slash_command, rename = "search-augment")]
async fn search_augment(
    ctx: HContext<'_>,
    #[description = "A name to search for."]
    name: Option<String>,
    #[description = "The allowed hull type."]
    hull_type: Option<EHullType>,
    #[description = "The rarity to select."]
    rarity: Option<EAugmentRarity>,
    #[description = "The name of the ship it is uniquely for."]
    #[autocomplete = "autocomplete::ship_name"]
    #[rename = "for-ship"]
    for_ship: Option<String>,
) -> HResult {
    use crate::buttons::azur::search_augment::*;

    let data = ctx.data();

    let unique_ship_id = match for_ship {
        Some(for_ship) => Some(find::ship(&data, &for_ship)?.group_id),
        None => None,
    };

    let filter = Filter {
        name,
        hull_type: hull_type.map(EHullType::convert),
        rarity: rarity.map(EAugmentRarity::convert),
        unique_ship_id,
    };

    let view = View::new(filter);
    ctx.send(view.modify(&data, ctx.create_reply())).await?;

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
) -> HResult {
    let reload_time = (200.0 / (100.0 + rld)).sqrt() * weapon_reload;

    let description = format!(
        "-# **Base Weapon FR:** {weapon_reload:.2}s \u{2E31} **`RLD:`**`{rld: >4}`\n\
         **Final FR:** {reload_time:.2}s"
    );

    let embed = CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .description(description);

    ctx.send(ctx.create_reply().embed(embed)).await?;
    Ok(())
}
