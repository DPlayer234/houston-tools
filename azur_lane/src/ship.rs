use std::fmt::Display;
use std::sync::Arc;
use serde::*;

use crate::define_data_enum;
use super::Faction;
use super::equip::*;
use super::skill::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipData {
    pub group_id: u32,
    pub name: Arc<str>,
    pub rarity: ShipRarity,
    pub faction: Faction,
    pub hull_type: HullType,
    pub stars: u8,
    #[serde(default)]
    pub enhance_kind: EnhanceKind,
    pub stats: ShipStats,
    pub equip_slots: Arc<[EquipSlot]>,
    pub shadow_equip: Arc<[ShadowEquip]>,
    pub skills: Arc<[Skill]>,
    pub retrofits: Arc<[ShipData]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wiki_name: Option<Arc<str>>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipStats {
    pub hp: f32,
    pub armor: ShipArmor,
    pub rld: f32,
    pub fp: f32,
    pub trp: f32,
    pub eva: f32,
    pub aa: f32,
    pub avi: f32,
    pub acc: f32,
    pub asw: f32,
    pub spd: f32,
    pub lck: f32,
    pub cost: u32,
    pub oxy: u32,
    pub amo: u32
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipSlot {
    pub allowed: Arc<[EquipKind]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mount: Option<EquipWeaponMount>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowEquip {
    pub name: Arc<str>,
    pub efficiency: f32,
    pub weapons: Arc<[Weapon]>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipWeaponMount {
    pub efficiency: f32,
    pub mounts: u8,
    pub parallel: u8,
    pub preload: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShipRarity {
    N,
    R,
    E,
    SR,
    UR
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum EnhanceKind {
    #[default]
    Normal,
    Research,
    META
}

define_data_enum! {
    pub enum StatKind for StatKindData {
        pub name: &'static str;

        HP("HP"),
        RLD("RLD"),
        FP("FP"),
        TRP("TRP"),
        EVA("EVA"),
        AA("AA"),
        AVI("AVI"),
        ACC("ACC"),
        ASW("ASW"),
        SPD("SPD"),
        LCK("LCK")
    }
}

define_data_enum! {
    pub enum HullType for HullTypeData {
        pub designation: &'static str,
        pub name: &'static str,
        pub team_type: TeamType;

        Unknown("??", "Unknown", TeamType::Vanguard),
        Destroyer("DD", "Destroyer", TeamType::Vanguard),
        LightCruiser("CL", "Light Cruiser", TeamType::Vanguard),
        HeavyCruiser("CA", "Heavy Cruiser", TeamType::Vanguard),
        Battlecruiser("BC", "Battlecruiser", TeamType::MainFleet),
        Battleship("BB", "Battleship", TeamType::MainFleet),
        LightCarrier("CVL", "Light Carrier", TeamType::MainFleet),
        AircraftCarrier("CV", "Aircraft Carrier", TeamType::MainFleet),
        Submarine("SS", "Submarine", TeamType::Submarine),
        AviationBattleship("BBV", "Aviation Battleship", TeamType::MainFleet),
        RepairShip("AR", "Repair Ship", TeamType::MainFleet),
        Monitor("BM", "Monitor", TeamType::MainFleet),
        AviationSubmarine("SSV", "Aviation Submarine", TeamType::Submarine),
        LargeCruiser("CB", "Large Cruiser", TeamType::Vanguard),
        MunitionShip("AE", "Munition Ship", TeamType::Vanguard),
        MissileDestroyerV("DDG v", "Missile Destroyer V", TeamType::Vanguard),
        MissileDestroyerM("DDG m", "Missile Destroyer M", TeamType::MainFleet),
        FrigateS("IX s", "Sailing Frigate S", TeamType::Submarine),
        FrigateV("IX v", "Sailing Frigate V", TeamType::Vanguard),
        FrigateM("IX m", "Sailing Frigate M", TeamType::MainFleet) 
    }
}

define_data_enum! {
    pub enum ShipArmor for ShipArmorData {
        pub name: &'static str;

        Light("Light"),
        Medium("Medium"),
        Heavy("Heavy")
    }
}

define_data_enum! {
    pub enum TeamType for TeamTypeData {
        pub name: &'static str;

        Vanguard("Vanguard"),
        MainFleet("Main Fleet"),
        Submarine("Submarine")
    }
}

impl Display for ShipArmor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.data().name)
    }
}

impl ShipRarity {
    pub fn next(self) -> Self {
        match self {
            Self::N => Self::R,
            Self::R => Self::E,
            Self::E => Self::SR,
            Self::SR | Self::UR => Self::UR,
        }
    }
}

impl ShipStats {
    pub fn multiply(&self, mult: f32) -> Self {
        Self {
            hp: self.hp * mult,
            armor: self.armor,
            rld: self.rld * mult,
            fp: self.fp * mult,
            trp: self.trp * mult,
            eva: self.eva * mult,
            aa: self.aa * mult,
            avi: self.avi * mult,
            acc: self.acc * mult,
            asw: self.asw * mult,
            spd: self.spd,
            lck: self.lck,
            cost: self.cost,
            oxy: self.oxy,
            amo: self.amo 
        }
    }

    pub fn get_stat(&self, kind: StatKind) -> f32 {
        match kind {
            StatKind::HP => self.hp,
            StatKind::RLD => self.rld,
            StatKind::FP => self.fp,
            StatKind::TRP => self.trp,
            StatKind::EVA => self.eva,
            StatKind::AA => self.aa,
            StatKind::AVI => self.avi,
            StatKind::ACC => self.acc,
            StatKind::ASW => self.asw,
            StatKind::SPD => self.spd,
            StatKind::LCK => self.lck
        }
    }
}
