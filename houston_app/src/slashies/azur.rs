use poise::ChoiceParameter;

use azur_lane::ship::*;
use azur_lane::Faction;

use crate::prelude::*;
use crate::buttons;
use crate::buttons::ButtonArgsModify;
use crate::buttons::azur::search_ship::*;

/// Information about mobile game Azur Lane.
#[poise::command(
    slash_command,
    subcommands("ship", "search_ship"),
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
    let ship = ctx.data().azur_lane().ships_by_prefix(&name).next().ok_or(buttons::azur::ShipParseError)?;
    let view = buttons::azur::ship::ViewShip::new(ship.group_id);
    ctx.send(view.modify_with_ship(ctx.data(), ctx.create_reply(), ship, None)).await?;
    Ok(())
}

/// Searches for ships.
#[poise::command(slash_command, rename = "search-ship")]
async fn search_ship(
    ctx: HContext<'_>,
    #[description = "A name prefix to search for."]
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

async fn autocomplete_name(ctx: HContext<'_>, partial: &str) -> Vec<String> {
    ctx.data().azur_lane()
        .ships_by_prefix(partial)
        .map(|s| (*s.name).to_owned())
        .take(10)
        .collect()
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
