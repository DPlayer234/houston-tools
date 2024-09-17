use poise::ChoiceParameter;

use azur_lane::ship::{HullType, ShipRarity};
use azur_lane::equip::{EquipKind, EquipRarity, AugmentRarity};
use azur_lane::Faction;

macro_rules! make_choice {
    ($NewType:ident for $OrigType:ident { $($(#[$attr:meta])* $name:ident),* $(,)? }) => {
        #[derive(ChoiceParameter)]
        pub enum $NewType {
            $(
                $(#[$attr])*
                $name
            ),*
        }

        impl $NewType {
            pub const fn convert(self) -> $OrigType {
                match self {
                    $( Self::$name => $OrigType::$name ),*
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
    #[name = "Iris Orthodoxy"] IrisOrthodoxy,
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
    #[name = "Collab: Senran Kagura"] CollabSenranKagura,
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
    #[name = "Sailing Frigate M"] FrigateM,
});

make_choice!(EShipRarity for ShipRarity {
    N, R, E, SR, UR,
});

make_choice!(EEquipKind for EquipKind {
    #[name = "DD Gun"] DestroyerGun,
    #[name = "CL Gun"] LightCruiserGun,
    #[name = "CA Gun"] HeavyCruiserGun,
    #[name = "CB Gun"] LargeCruiserGun,
    #[name = "BB Gun"] BattleshipGun,
    #[name = "Torpedo (Surface)"] SurfaceTorpedo,
    #[name = "Torpedo (Submarine)"] SubmarineTorpedo,
    #[name = "Anti-Air Gun"] AntiAirGun,
    #[name = "Anti-Air Gun (Fuze)"] FuzeAntiAirGun,
    #[name = "Fighter"] Fighter,
    #[name = "Dive Bomber"] DiveBomber,
    #[name = "Torpedo Bomber"] TorpedoBomber,
    #[name = "Seaplane"] SeaPlane,
    #[name = "Anti-Sub Weapon"] AntiSubWeapon,
    #[name = "Anti-Sub Aircraft"] AntiSubAircraft,
    #[name = "Helicopter"] Helicopter,
    #[name = "Missile"] Missile,
    #[name = "Cargo"] Cargo,
    #[name = "Auxiliary"] Auxiliary,
});

make_choice!(EEquipRarity for EquipRarity {
    #[name = "1* Common"] N1,
    #[name = "2* Common"] N2,
    #[name = "3* Rare"] R,
    #[name = "4* Elite"] E,
    #[name = "5* SR"] SR,
    #[name = "6* UR"] UR,
});

make_choice!(EAugmentRarity for AugmentRarity {
    #[name = "2* Rare"] R,
    #[name = "3* Elite"] E,
    #[name = "4* SR"] SR,
});
