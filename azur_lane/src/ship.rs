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
    pub hp: ShipStatValue,
    pub armor: ShipArmor,
    pub rld: ShipStatValue,
    pub fp: ShipStatValue,
    pub trp: ShipStatValue,
    pub eva: ShipStatValue,
    pub aa: ShipStatValue,
    pub avi: ShipStatValue,
    pub acc: ShipStatValue,
    pub asw: ShipStatValue,
    pub spd: f32,
    pub lck: f32,
    pub cost: u32,
    pub oxy: u32,
    pub amo: u32
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ShipStatValue(f32, f32, f32);

impl ShipStatValue {
    pub const fn new(base: f32, growth: f32, fixed: f32) -> Self {
        Self(base, growth, fixed)
    }

    pub const fn base(&self) -> f32 { self.0 }
    pub const fn growth(&self) -> f32 { self.1 }
    pub const fn fixed(&self) -> f32 { self.2 }

    pub fn calc(&self, level: u32, affinity: f32) -> f32 {
        (self.base() + self.growth() * ((level - 1) as f32) * 0.001f32) * affinity + self.fixed()
    }
}

impl std::ops::Add<Self> for ShipStatValue {
    type Output = Self;

    fn add(self, rhs: ShipStatValue) -> Self::Output {
        Self(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }
}

impl std::ops::AddAssign for ShipStatValue {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs
    }
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

define_data_enum! {
    pub enum ShipRarity for ShipRarityData {
        pub name: &'static str,
        pub color_rgb: u32;

        N("N", 0xC0C0C0),
        R("R", 0x9FE8FF),
        E("E", 0xC4ADFF),
        SR("SR", 0xEDDD76),
        UR("UR", 0xFF8D8D)
    }
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
    pub fn get_stat(&self, kind: StatKind) -> f32 {
        match kind {
            StatKind::HP => self.hp.calc(125, 1.0),
            StatKind::RLD => self.rld.calc(125, 1.0),
            StatKind::FP => self.fp.calc(125, 1.0),
            StatKind::TRP => self.trp.calc(125, 1.0),
            StatKind::EVA => self.eva.calc(125, 1.0),
            StatKind::AA => self.aa.calc(125, 1.0),
            StatKind::AVI => self.avi.calc(125, 1.0),
            StatKind::ACC => self.acc.calc(125, 1.0),
            StatKind::ASW => self.asw.calc(125, 1.0),
            StatKind::SPD => self.spd,
            StatKind::LCK => self.lck
        }
    }
}
