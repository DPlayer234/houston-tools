use poise::ChoiceParameter;

use azur_lane::ship::*;
use azur_lane::equip::*;
use azur_lane::Faction;

use crate::prelude::*;
use crate::buttons;
use crate::buttons::ButtonArgsModify;

/// Information about mobile game Azur Lane.
#[poise::command(
    slash_command,
    subcommands("ship", "search_ship", "equip", "search_equip"),
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
    #[autocomplete = "autocomplete_ship_name"]
    name: String
) -> HResult {
    let ship = name.parse().map(|id| ctx.data().azur_lane().ship_by_id(id))
        .unwrap_or_else(|_| ctx.data().azur_lane().ships_by_prefix(&name).next())
        .ok_or(buttons::azur::EquipParseError)?;

    let view = buttons::azur::ship::ViewShip::new(ship.group_id);
    ctx.send(view.modify_with_ship(ctx.data(), ctx.create_reply(), ship, None)).await?;
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
    hull_type: Option<EHullType>,
    #[description = "The rarity to select."]
    rarity: Option<EShipRarity>,
    #[description = "Whether the ships have a unique augment."]
    has_augment: Option<bool>
) -> HResult {
    use crate::buttons::azur::search_ship::*;

    let filter = Filter {
        name,
        faction: faction.map(EFaction::convert),
        hull_type: hull_type.map(EHullType::convert),
        rarity: rarity.map(EShipRarity::convert),
        has_augment
    };

    let view = ViewSearchShip::new(filter);
    ctx.send(view.modify(ctx.data(), ctx.create_reply())?).await?;

    Ok(())
}

/// Shows information about equipment.
#[poise::command(slash_command)]
async fn equip(
    ctx: HContext<'_>,
    #[description = "The equipment name. This supports auto completion."]
    #[autocomplete = "autocomplete_equip_name"]
    name: String
) -> HResult {
    let equip = name.parse().map(|id| ctx.data().azur_lane().equip_by_id(id))
        .unwrap_or_else(|_| ctx.data().azur_lane().equips_by_prefix(&name).next())
        .ok_or(buttons::azur::EquipParseError)?;

    let view = buttons::azur::equip::ViewEquip::new(equip.equip_id);
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

    let filter = Filter {
        name,
        faction: faction.map(EFaction::convert),
        kind: kind.map(EEquipKind::convert),
        rarity: rarity.map(EEquipRarity::convert),
    };

    let view = ViewSearchEquip::new(filter);
    ctx.send(view.modify(ctx.data(), ctx.create_reply())?).await?;

    Ok(())
}

async fn autocomplete_ship_name<'a>(ctx: HContext<'a>, partial: &'a str) -> impl Iterator<Item = AutocompleteChoice> + 'a {
    ctx.data().azur_lane()
        .ships_by_prefix(partial)
        .map(|s| AutocompleteChoice::new(s.name.as_str(), s.group_id.to_string()))
}

async fn autocomplete_equip_name<'a>(ctx: HContext<'a>, partial: &'a str) -> impl Iterator<Item = AutocompleteChoice> + 'a {
    ctx.data().azur_lane()
        .equips_by_prefix(partial)
        .map(|s| AutocompleteChoice::new(s.name.as_str(), s.equip_id.to_string()))
}

macro_rules! make_choice {
    ($NewType:ident for $OrigType:ident { $($(#[$attr:meta])* $name:ident),* }) => {
        #[derive(ChoiceParameter)]
        enum $NewType {
            $(
                $(#[$attr])*
                $name
            ),*
        }

        impl $NewType {
            fn convert(self) -> $OrigType {
                match self {
                    $(
                        Self::$name => $OrigType::$name
                    ),*
                }
            }
        }
    };
}

make_choice!(EFaction for Faction {
    Unknown,
    Universal,
    #[name = "Eagle Union"] EagleUnion,
    #[name = "Royal Navy"] RoyalNavy,
    #[name = "Sakura Empire"] SakuraEmpire,
    #[name = "Iron Blood"] IronBlood,
    #[name = "Dragon Empery"] DragonEmpery,
    #[name = "Sardegna Empire"] SardegnaEmpire,
    #[name = "Northern Parliament"] NorthernParliament,
    #[name = "Iris Libre"] IrisLibre,
    #[name = "Vichya Dominion"] VichyaDominion,
    Tempesta,
    META,
    #[name = "Collab: Neptunia"] CollabNeptunia,
    #[name = "Collab: Bilibili"] CollabBilibili,
    #[name = "Collab: Utawarerumono"] CollabUtawarerumono,
    #[name = "Collab: Kizuna AI"] CollabKizunaAI,
    #[name = "Collab: Hololive"] CollabHololive,
    #[name = "Collab: Venus Vacation"] CollabVenusVacation,
    #[name = "Collab: Idolm@ster"] CollabIdolmaster,
    #[name = "Collab: SSSS"] CollabSSSS,
    #[name = "Collab: Atelier Ryza"] CollabAtelierRyza,
    #[name = "Collab: Senran Kagura"] CollabSenranKagura
});

make_choice!(EHullType for HullType {
    #[name = "Unknown"] Unknown,
    #[name = "Destroyer"] Destroyer,
    #[name = "Light Cruiser"] LightCruiser,
    #[name = "Heavy Cruiser"] HeavyCruiser,
    #[name = "Battlecruiser"] Battlecruiser,
    #[name = "Battleship"] Battleship,
    #[name = "Light Carrier"] LightCarrier,
    #[name = "Aircraft Carrier"] AircraftCarrier,
    #[name = "Submarine"] Submarine,
    #[name = "Aviation Battleship"] AviationBattleship,
    #[name = "Repair Ship"] RepairShip,
    #[name = "Monitor"] Monitor,
    #[name = "Aviation Submarine"] AviationSubmarine,
    #[name = "Large Cruiser"] LargeCruiser,
    #[name = "Munition Ship"] MunitionShip,
    #[name = "Missile Destroyer V"] MissileDestroyerV,
    #[name = "Missile Destroyer M"] MissileDestroyerM,
    #[name = "Sailing Frigate S"] FrigateS,
    #[name = "Sailing Frigate V"] FrigateV,
    #[name = "Sailing Frigate M"] FrigateM
});

make_choice!(EShipRarity for ShipRarity {
    N, R, E, SR, UR
});

make_choice!(EEquipKind for EquipKind {
    #[name = "DD Gun"]
    DestroyerGun,
    #[name = "CL Gun"]
    LightCruiserGun,
    #[name = "CA Gun"]
    HeavyCruiserGun,
    #[name = "CB Gun"]
    LargeCruiserGun,
    #[name = "BB Gun"]
    BattleshipGun,
    #[name = "Torpedo (Surface)"]
    SurfaceTorpedo,
    #[name = "Torpedo (Submarine)"]
    SubmarineTorpedo,
    #[name = "Anti-Air Gun"]
    AntiAirGun,
    #[name = "Anti-Air Gun (Fuze)"]
    FuzeAntiAirGun,
    #[name = "Fighter"]
    Fighter,
    #[name = "Dive Bomber"]
    DiveBomber,
    #[name = "Torpedo Bomber"]
    TorpedoBomber,
    #[name = "Seaplane"]
    SeaPlane,
    #[name = "Anti-Sub Weapon"]
    AntiSubWeapon,
    #[name = "Anti-Sub Aircraft"]
    AntiSubAircraft,
    #[name = "Helicopter"]
    Helicopter,
    #[name = "Missile"]
    Missile,
    #[name = "Cargo"]
    Cargo,
    #[name = "Auxiliary"]
    Auxiliary
});

make_choice!(EEquipRarity for EquipRarity {
    #[name = "1* Common"]
    N1,
    #[name = "2* Common"]
    N2,
    #[name = "3* Rare"]
    R,
    #[name = "4* Elite"]
    E,
    #[name = "5* SR"]
    SR,
    #[name = "6* UR"]
    UR
});
