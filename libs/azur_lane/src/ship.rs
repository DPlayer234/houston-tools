//! Data structures relating directly to ships.

use std::fmt;
use std::ops::{Add, AddAssign};

use serde::{Deserialize, Serialize};
use small_fixed_array::{FixedArray, FixedString};

use crate::data_def::is_default;
use crate::equip::*;
use crate::skill::*;
use crate::{Faction, GameServer, define_data_enum};

/// Provides data for a singular ship or a retrofit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipData {
    /// The group ID. This is the same for the base and its retrofits.
    pub group_id: u32,
    /// The ship's display name.
    pub name: FixedString,
    /// The ship's rarity.
    ///
    /// For its star rating, see [`ShipData::stars`].
    pub rarity: ShipRarity,
    /// The faction this ship belongs to.
    pub faction: Faction,
    /// The hull type/designation.
    pub hull_type: HullType,
    /// The star rating for the ship.
    pub stars: u8,
    /// How the ship is enhanced.
    #[serde(default)]
    pub enhance_kind: EnhanceKind,
    /// The ship's stats.
    pub stats: ShipStatBlock,
    /// The ID of the default skin.
    /// Retrofits will have the retrofit skin set as the default.
    ///
    /// [`ShipData::skin_by_id`] can be used to easily get skin data.
    pub default_skin_id: u32,
    /// The real equipment slots visible in-game, including auxiliary slots.
    pub equip_slots: FixedArray<EquipSlot>,
    /// Additional shadow or hidden equipment that's fixed to the ship.
    ///
    /// Most commonly, this is a secondary gun for torpedo CLs or CAs.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub shadow_equip: FixedArray<ShadowEquip>,
    /// Default equipped depth charges.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub depth_charges: FixedArray<Equip>,
    /// The list of skills. Excludes inactive or hidden skills.
    pub skills: FixedArray<Skill>,
    /// The ultimate bonus this ship gets upon max limit break. Currently, this
    /// is only set for Destroyers and Harbin.
    ///
    /// Also referred to as the "specific type".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ultimate_bonus: Option<UltimateBonus>,
    /// Available retrofits for this ship in their maxed-out state.
    ///
    /// As of now, only DDGs have "multiple" retrofits, with their vanguard
    /// and main fleet states being considered different ones.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub retrofits: FixedArray<ShipData>,
    /// The ship's skins, including their default and all retrofit skins.
    ///
    /// This will be empty for nested retrofits. Access the base's skins.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub skins: FixedArray<ShipSkin>,
    /// The fleet tech bonuses for this ship.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fleet_tech: Option<FleetTechInfo>,
}

/// Provides stat block information for a ship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipStatBlock {
    pub hp: ShipStat,
    pub armor: ShipArmor,
    pub rld: ShipStat,
    pub fp: ShipStat,
    pub trp: ShipStat,
    pub eva: ShipStat,
    pub aa: ShipStat,
    pub avi: ShipStat,
    pub acc: ShipStat,
    pub asw: ShipStat,
    pub spd: f64,
    pub lck: f64,
    pub cost: u32,
    pub oxy: u32,
    pub amo: u32,
}

/// Represents a single ship stat. Its value can be calculated on demand.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct ShipStat(f64, f64, f64);

/// A singular normal equipment slot of a ship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipSlot {
    /// Which kinds of equipment can be equipped in the slot.
    pub allowed: FixedArray<EquipKind>,
    /// If a weapon slot, the data for the mount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mount: Option<EquipWeaponMount>,
}

/// Mount information for an [`EquipSlot`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipWeaponMount {
    /// The mount efficiency, as displayed in-game.
    pub efficiency: f64,
    /// The amount of mounts.
    pub mounts: u8,
    /// The amount of parallel loads.
    ///
    /// F.e. Gascogne's main gun and Unzen's torpedo have a value of 2.
    pub parallel: u8,
    /// How many preloads this slot has.
    ///
    /// This is only meaningful for Battleship main guns, torpedoes, and
    /// missiles.
    #[serde(default, skip_serializing_if = "is_default")]
    pub preload: u8,
    /// How many retriggers the gun has on fire.
    ///
    /// This is only meaningful for Battleship main guns.
    #[serde(default, skip_serializing_if = "is_default")]
    pub retriggers: u8,
}

/// Provides information about "shadow" equipment; inherent gear that is not
/// displayed in-game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowEquip {
    /// The name of the associated equipment.
    pub name: FixedString,
    /// The mount efficiency. Same meaning as [`EquipWeaponMount::efficiency`].
    pub efficiency: f64,
    /// The weapons on that equipment.
    pub weapons: FixedArray<Weapon>,
}

/// Data for a ship skin. This may represent the default skin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipSkin {
    /// The skin's ID. [`ShipData::skin_by_id`] searches for this.
    pub skin_id: u32,
    /// The image/asset key.
    ///
    /// Asset bundles and chibi sprites from the collector will use this as
    /// their filename.
    pub image_key: FixedString,
    /// The skin's display name.
    pub name: FixedString,
    /// The skin's description.
    pub description: FixedString,
    /// The default dialogue lines.
    ///
    /// This has one entry per game server. Which server each entry belongs to
    /// is indicated by [`ShipSkinWords::server`].
    pub words: FixedArray<ShipSkinWords>,
    /// Replacement dialogue lines, usually after oath.
    ///
    /// This has one entry per game server. Which server each entry belongs to
    /// is indicated by [`ShipSkinWords::server`].
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub words_extra: FixedArray<ShipSkinWords>,
}

/// The block of dialogue for a given skin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipSkinWords {
    /// The server with these words.
    pub server: GameServer,
    /// The skin's description.
    ///
    /// Note that [`ShipSkin::description`] originates from the skin's template,
    /// whereas this field is actually part of the skin's words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<FixedString>,
    /// The "introduction". In-game, this is the profile text in the archive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub introduction: Option<FixedString>,
    /// Dialogue played when the ship is obtained.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquisition: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<FixedString>,
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub main_screen: FixedArray<ShipMainScreenLine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub touch: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub special_touch: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rub: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission_reminder: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission_complete: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mail_reminder: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_to_port: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commission_complete: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enhance: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flagship_fight: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub victory: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defeat: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low_health: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disappointed: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stranger: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub friendly: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crush: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub love: Option<FixedString>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oath: Option<FixedString>,
    /// Voices lines that may be played when sortieing other specific ships.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub couple_encourage: FixedArray<ShipCoupleEncourage>,
}

/// Information about a ship line that may be displayed on the main screen.
///
/// Also see [`ShipSkinWords::main_screen`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipMainScreenLine(usize, FixedString);

/// Data for voices lines that may be played when sortieing other specific
/// ships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipCoupleEncourage {
    /// The line to be played.
    pub line: FixedString,
    /// The amount of allies that need to match the condition.
    pub amount: u32,
    /// The condition rule.
    pub condition: ShipCouple,
}

/// Condition for [`ShipCoupleEncourage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShipCouple {
    /// Triggered when other specific ships are present.
    /// Holds a vector of ship group IDs.
    ShipGroup(FixedArray<u32>),
    /// Triggered when ships of specified hull types are present.
    HullType(FixedArray<HullType>),
    /// Triggered when ships of a specified rarity are present.
    Rarity(FixedArray<ShipRarity>),
    /// Triggered when ships from a specified faction are present.
    Faction(FixedArray<Faction>),
    /// Triggered when ships from the same illustrator are present.
    ///
    /// Actual in-game data specifies which one, but it's only ever used to
    /// refer to the same one as the source ship's.
    Illustrator,
    /// Triggered based on team type. Unused?
    Team,
    /// Unknown trigger types.
    Unknown,
}

/// Information about fleet tech bonuses for a ship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetTechInfo {
    // `class`, info can be looked up in `fleet_tech_ship_class`
    pub class: u32,

    /// The amount of PTs gained when getting the ship.
    pub pt_get: u32,
    /// The amount of PTs gained for reaching level 120 with the ship.
    pub pt_level: u32,
    /// The amount of PTs gained for fully limit breaking the ship.
    pub pt_limit_break: u32,

    /// The stat bonuses gained when getting the ship.
    pub stats_get: FleetTechStatBonus,
    /// The stat bonuses gained when reaching level 120 with the ship.
    pub stats_level: FleetTechStatBonus,
}

/// A stat bonus gained via ship fleet tech.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetTechStatBonus {
    /// The hull types that are affected by this bonus.
    pub hull_types: FixedArray<HullType>,
    /// The stat that is affected by this bonus.
    pub stat: StatKind,
    /// The amount of fixed stats gained by this bonus.
    pub amount: f64,
}

define_data_enum! {
    /// The rarities for a ship.
    pub enum ShipRarity for ShipRarityData {
        /// The display name for the rarity.
        pub name: &'static str,
        /// An RGB color that can be used to represent the rarity.
        pub color_rgb: u32;

        /// N (Common)
        N("N", 0xC0C0C0),
        /// R (Rare)
        R("R", 0x9FE8FF),
        /// E (Elite)
        E("E", 0xC4ADFF),
        /// SR (Super Rare) / Priority
        SR("SR", 0xEDDD76),
        /// UR (Ultra Rare) / Decisive
        UR("UR", 0xFF8D8D)
    }
}

/// The enhancement mode kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum EnhanceKind {
    /// Normal. Enhancement by feeding spare duplicate ships.
    #[default]
    Normal,
    /// Research ships enhanced with blueprints.
    Research,
    /// META ships with their own nonsense.
    Meta,
}

define_data_enum! {
    /// The possible stat kinds.
    ///
    /// Only includes ones that represent a numeric value.
    pub enum StatKind for StatKindData {
        /// The in-game display name.
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
    /// The possible hull types/designations for ships.
    pub enum HullType for HullTypeData {
        /// The short-hand designation for the hull type.
        pub designation: &'static str,
        /// The long hull type name.
        pub name: &'static str,
        /// Which team type this hull type gets sortied in.
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
        MissileDestroyerV("DDGv", "Missile Destroyer V", TeamType::Vanguard),
        MissileDestroyerM("DDGm", "Missile Destroyer M", TeamType::MainFleet),
        FrigateS("IXs", "Sailing Frigate S", TeamType::Submarine),
        FrigateV("IXv", "Sailing Frigate V", TeamType::Vanguard),
        FrigateM("IXm", "Sailing Frigate M", TeamType::MainFleet)
    }
}

define_data_enum! {
    /// The armor thickness of a ship.
    pub enum ShipArmor for ShipArmorData {
        /// The display name for the armor type.
        pub name: &'static str;

        Light("Light"),
        Medium("Medium"),
        Heavy("Heavy")
    }
}

define_data_enum! {
    /// The sortie team types.
    pub enum TeamType for TeamTypeData {
        /// The display name for the team type.
        pub name: &'static str;

        Vanguard("Vanguard"),
        MainFleet("Main Fleet"),
        Submarine("Submarine Fleet")
    }
}

define_data_enum! {
    /// The kind of "ultimate bonus" a ship gets upon max limit break.
    pub enum UltimateBonus for UltimateBonusData {
        /// The description for the type.
        pub description: &'static str;

        Unknown("Unknown `specific_type`"),
        Auxiliary("+30% stats gained from auxiliary gear"),
        Torpedo("Decrease torpedo spread angle"),
        Gunner("Halve shots needed to activate All Out Assault")
    }
}

impl fmt::Display for ShipArmor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl ShipData {
    /// Gets a skin for this ship by its ID.
    ///
    /// Retrofits will have empty skin lists. Call this on the base ship.
    #[must_use]
    pub fn skin_by_id(&self, skin_id: u32) -> Option<&ShipSkin> {
        self.skins.iter().find(|s| s.skin_id == skin_id)
    }
}

impl ShipStatBlock {
    /// Gets and calculates a certain stat value.
    ///
    /// Refer to [`ShipStat::calc`] for potential caveats.
    #[must_use]
    pub fn calc_stat(&self, kind: StatKind, level: u32, affinity: f64) -> f64 {
        match kind {
            StatKind::HP => self.hp.calc(level, affinity),
            StatKind::RLD => self.rld.calc(level, affinity),
            StatKind::FP => self.fp.calc(level, affinity),
            StatKind::TRP => self.trp.calc(level, affinity),
            StatKind::EVA => self.eva.calc(level, affinity),
            StatKind::AA => self.aa.calc(level, affinity),
            StatKind::AVI => self.avi.calc(level, affinity),
            StatKind::ACC => self.acc.calc(level, affinity),
            StatKind::ASW => self.asw.calc(level, affinity),
            StatKind::SPD => self.spd,
            StatKind::LCK => self.lck,
        }
    }
}

impl ShipStat {
    /// Creates a stat with all zeroes.
    #[must_use]
    pub const fn new() -> Self {
        Self(0f64, 0f64, 0f64)
    }

    /// Sets the base value.
    #[must_use]
    pub const fn with_base(mut self, base: f64) -> Self {
        self.0 = base;
        self
    }

    /// Sets the level growth value.
    #[must_use]
    pub const fn with_growth(mut self, growth: f64) -> Self {
        self.1 = growth;
        self
    }

    /// Sets the fixed addition unaffected by affinity.
    #[must_use]
    pub const fn with_fixed(mut self, fixed: f64) -> Self {
        self.2 = fixed;
        self
    }

    /// The base value.
    ///
    /// This isn't the level 1 value and includes various enhancements.
    /// See also: [`ShipStat::calc`]
    pub const fn base(&self) -> f64 {
        self.0
    }

    /// The level growth value.
    pub const fn growth(&self) -> f64 {
        self.1
    }

    /// A fixed addition unaffected by affinity.
    pub const fn fixed(&self) -> f64 {
        self.2
    }

    /// Calculates the actual value.
    ///
    /// It should be noted that, due to the way this is generally stored, asking
    /// for levels below 100 will lead to inaccurate results. In particular,
    /// stats from Limit Breaks, Enhancement, Dev, Fate Simulation, and META
    /// Repair always represent the maxed state.
    #[must_use]
    pub fn calc(&self, level: u32, affinity: f64) -> f64 {
        (self.base() + self.growth() * f64::from(level - 1) * 0.001) * affinity + self.fixed()
    }
}

utils::impl_op_via_assign!(copy ShipStat, [AddAssign]::add_assign, [Add]::add);

impl AddAssign<&Self> for ShipStat {
    fn add_assign(&mut self, rhs: &Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
        self.2 += rhs.2;
    }
}

impl ShipSkin {
    pub fn words(&self, server: GameServer) -> Option<(&ShipSkinWords, Option<&ShipSkinWords>)> {
        let main = self
            .words
            .iter()
            .find(|s| s.server == server)
            .or_else(|| self.words.first())?;

        let extra = self
            .words_extra
            .iter()
            .find(|s| s.server == server)
            .or_else(|| self.words_extra.first());

        Some((main, extra))
    }
}

impl ShipMainScreenLine {
    /// Creates a new instance.
    #[must_use]
    pub fn new(index: usize, text: FixedString) -> Self {
        Self(index, text)
    }

    /// Gets the index for the line. Relevant for replacement.
    pub fn index(&self) -> usize {
        self.0
    }

    /// Gets the text associated with the line.
    pub fn text(&self) -> &str {
        &self.1
    }

    /// Sets the index for the line.
    #[must_use]
    pub fn with_index(self, index: usize) -> Self {
        Self(index, self.1)
    }
}

impl ShipRarity {
    /// Returns the next higher rarity.
    ///
    /// For [`ShipRarity::UR`], returns itself.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::N => Self::R,
            Self::R => Self::E,
            Self::E => Self::SR,
            Self::SR | Self::UR => Self::UR,
        }
    }
}
