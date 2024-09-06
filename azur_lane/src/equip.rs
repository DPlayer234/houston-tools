//! Provides data structures for ship equipment.

use serde::{Serialize, Deserialize};

use crate::define_data_enum;
use crate::ship::*;
use crate::skill::*;
use super::Faction;

/// Represents a piece of equipment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equip {
    pub equip_id: u32,
    pub name: String,
    pub description: String,
    pub kind: EquipKind,
    pub rarity: EquipRarity,
    pub faction: Faction,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub hull_allowed: Vec<HullType>,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub hull_disallowed: Vec<HullType>,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub weapons: Vec<Weapon>,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<Skill>,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub stat_bonuses: Vec<EquipStatBonus>
}

/// A weapon that is part of [`Equip`] or [`Skill`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weapon {
    pub weapon_id: u32,
    pub name: Option<String>,
    pub reload_time: f64,
    pub fixed_delay: f64,
    pub kind: WeaponKind,
    pub data: WeaponData
}

/// A bullet barrage pattern for a [`Weapon`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Barrage {
    pub damage: f64,
    pub coefficient: f64,
    pub scaling: f64,
    pub scaling_stat: StatKind,
    pub range: f64,
    pub firing_angle: f64,
    pub salvo_time: f64,
    pub bullets: Vec<Bullet>
}

/// Bullet information for a [`Barrage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bullet {
    pub bullet_id: u32,

    // From barrage template:
    pub amount: u32,

    // From bullet template:
    pub kind: BulletKind,
    pub ammo: AmmoKind,
    pub pierce: u32,
    pub velocity: f64,
    pub modifiers: ArmorModifiers,
    pub flags: BulletFlags,

    /// Buffs caused by the bullet hit.
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub attach_buff: Vec<BuffInfo>,

    /// Extra data depending on the bullet type.
    #[serde(default, skip_serializing_if = "is_none_bullet_extra")]
    pub extra: BulletExtra,
}

fn is_none_bullet_extra(extra: &BulletExtra) -> bool {
    matches!(extra, BulletExtra::None)
}

/// Additional bullet data.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BulletExtra {
    #[default] None,
    Spread(BulletSpread),
    Beam(BulletBeam),
}

/// How far a bullet's hit spread is. Only applicable to main gun fire and bombs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletSpread {
    pub spread_x: f64,
    pub spread_y: f64,
    pub hit_range: f64,
}

/// Additional information about a beam.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletBeam {
    pub duration: f64,
    pub tick_delay: f64,
}

/// Aircraft data for a [`Weapon`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aircraft {
    pub aircraft_id: u32,
    pub amount: u32,
    pub speed: f64,
    pub health: ShipStat,
    pub dodge_limit: u32,
    pub weapons: Vec<Weapon>,
}

/// The possible data a [`Weapon`] can hold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WeaponData {
    /// The weapon fires bullets as a [`Barrage`].
    Bullets(Barrage),
    /// The weapon launches an [`Aircraft`].
    Aircraft(Aircraft),
    /// The weapon fires anti-air attacks as a [`Barrage`].
    AntiAir(Barrage),
}

/// Armor modifiers to apply to the damage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArmorModifiers(pub f64, pub f64, pub f64);

/// Bonus stats gained by equipping the associated equipment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipStatBonus {
    pub stat_kind: StatKind,
    pub amount: f64
}

/// Represents an Augment Module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Augment {
    pub augment_id: u32,
    pub name: String,
    pub rarity: AugmentRarity,
    pub stat_bonuses: Vec<AugmentStatBonus>,
    pub allowed: Vec<HullType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<Skill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unique_ship_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_upgrade: Option<AugmentSkillUpgrade>,
}

/// Bonus stats gained by equipping the associated augment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentStatBonus {
    pub stat_kind: StatKind,
    pub amount: f64,
    pub random: f64
}

/// A skill upgraded by an augment module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentSkillUpgrade {
    pub original_id: u32,
    pub skill: Skill,
}

bitflags::bitflags! {
    /// Additional flags for a bullet.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct BulletFlags: u8 {
        const IGNORE_SHIELD = 1 << 0;
        const IGNORE_SURFACE = 1 << 1;
        const IGNORE_DIVE = 1 << 2;
    }
}

impl BulletFlags {
    pub fn dive_filter(self) -> Self {
        self & (BulletFlags::IGNORE_SURFACE | BulletFlags::IGNORE_DIVE)
    }
}

define_data_enum! {
    /// The possible kinds of equipment.
    pub enum EquipKind for EquipKindData {
        pub name: &'static str;

        DestroyerGun("DD Gun"),
        LightCruiserGun("CL Gun"),
        HeavyCruiserGun("CA Gun"),
        LargeCruiserGun("CB Gun"),
        BattleshipGun("BB Gun"),
        SurfaceTorpedo("Torpedo (Surface)"),
        SubmarineTorpedo("Torpedo (Submarine)"),
        AntiAirGun("Anti-Air Gun"),
        FuzeAntiAirGun("Anti-Air Gun (Fuze)"),
        Fighter("Fighter"),
        DiveBomber("Dive Bomber"),
        TorpedoBomber("Torpedo Bomber"),
        SeaPlane("Seaplane"),
        AntiSubWeapon("Anti-Sub Weapon"),
        AntiSubAircraft("Anti-Sub Aircraft"),
        Helicopter("Helicopter"),
        Missile("Missile"),
        Cargo("Cargo"),
        Auxiliary("Auxiliary")
    }
}

define_data_enum! {
    /// The possible kinds of bullets.
    pub enum BulletKind for BulletKindData {
        /// A friendly name for the bullet kind.
        pub name: &'static str;

        Cannon("Cannon"),
        Bomb("Bomb"),
        Torpedo("Torpedo"),
        Direct("Direct"),
        Shrapnel("Shrapnel"),
        AntiAir("Anti-Air"),
        AntiSea("Anti-Submarine"),
        Effect("Effect"),
        Beam("Beam"),
        GBullet("GBullet"),
        EletricArc("Eletric Arc"),
        Missile("Missile"),
        SpaceLaser("Space Laser"),
        Scale("Scale"),
        TriggerBomb("Trigger Bomb"),
        AAMissile("AA Missile")
    }
}

define_data_enum! {
    /// The possible kinds of ammo.
    pub enum AmmoKind for AmmoKindData {
        /// The full ammo name.
        pub name: &'static str,
        /// A shorter ammo name.
        pub short_name: &'static str;

        Normal("Normal", "Nor."),
        AP("AP", "AP"),
        HE("HE", "HE"),
        Torpedo("Torpedo", "Tor."),
        AirToAir("Air-to-Air", "Air."),
        Bomb("Bomb", "Bomb"),
        SAP("SAP", "SAP"),
        Unknown8("8", "?"),
        Unknown9("9", "?")
    }
}

define_data_enum! {
    pub enum WeaponKind for WeaponKindData {
        pub name: &'static str;

        MainGun("Main Gun"),
        SubGun("Auto Gun"),
        Torpedo("Torpedo"),
        AirToAir("Anti-Air"),
        Armor("Armor"),
        Engine("Engine"),
        Radar("Radar"),
        StrikeAircraft("Aircraft"),
        InterceptAircraft("Aircraft (Intercept)"),
        Crew("Crew"),
        Charge("Charge"),
        Special("Special"),
        MegaCharge("Mega Charge"),
        ManualTorpedo("Torpedo (Manual)"),
        AntiSub("Aircraft (Anti-Sub)"),
        HammerHead("Hammer Head"),
        BomberPreCastAlert("Bomber Pre-Cast Alert"),
        MultiLock("Multi-Lock"),
        ManualSub("Anti-Sub (Manual)"),
        AntiAir("Anti-Air"),
        Bracketing("Main Gun (Bracketing)"),
        Beam("Beam"),
        DepthCharge("Depth Charge"),
        AntiAirRepeater("Anti-Air (Repeater)"),
        DisposableTorpedo("Torpedo (Disposable)"),
        SpaceLaser("Space Laser"),
        Missile("Missile??"),
        AntiAirFuze("Anti-Air (Fuze)"),
        ManualMissile("Missile (Manual)"),
        AutoMissile("Missile (Auto)"),
        Meteor("Meteor"),
        Unknown("Unknown")
    }
}

define_data_enum! {
    /// The rarities for equip.
    pub enum EquipRarity for EquipRarityData {
        pub stars: u32,
        /// The display name for the rarity.
        pub name: &'static str,
        /// An RGB color that can be used to represent the rarity.
        pub color_rgb: u32;

        /// 1* (Common)
        N1(1, "N", 0xC0C0C0),
        /// 2* (Common)
        N2(2, "N", 0xC0C0C0),
        /// 3* R (Rare)
        R(3, "R", 0x9FE8FF),
        /// 4* E (Elite)
        E(4, "E", 0xC4ADFF),
        /// 5* SR (Super Rare)
        SR(5, "SR", 0xEDDD76),
        /// 6* UR (Ultra Rare)
        UR(6, "UR", 0xFF8D8D)
    }
}

define_data_enum! {
    /// The rarities for augments.
    pub enum AugmentRarity for AugmentRarityData {
        pub stars: u32,
        /// The display name for the rarity.
        pub name: &'static str,
        /// An RGB color that can be used to represent the rarity.
        pub color_rgb: u32;

        /// 2* R (Rare)
        R(2, "R", 0x9FE8FF),
        /// 3* E (Elite)
        E(3, "E", 0xC4ADFF),
        /// 4* SR (Super Rare)
        SR(4, "SR", 0xEDDD76)
    }
}

impl ArmorModifiers {
    /// Gets the modifier for a specific kind of armor.
    pub fn modifier(&self, armor_kind: ShipArmor) -> f64 {
        match armor_kind {
            ShipArmor::Light => self.0,
            ShipArmor::Medium => self.1,
            ShipArmor::Heavy => self.2,
        }
    }

    /// Sets the modifier for a specific kind of armor.
    #[must_use]
    pub fn with_modifier(mut self, armor_kind: ShipArmor, value: f64) -> Self {
        *match armor_kind {
            ShipArmor::Light => &mut self.0,
            ShipArmor::Medium => &mut self.1,
            ShipArmor::Heavy => &mut self.2,
        } = value;
        self
    }
}

impl From<[f64; 3]> for ArmorModifiers {
    fn from(value: [f64; 3]) -> Self {
        ArmorModifiers(value[0], value[1], value[2])
    }
}

impl From<(f64, f64, f64)> for ArmorModifiers {
    fn from(value: (f64, f64, f64)) -> Self {
        ArmorModifiers(value.0, value.1, value.2)
    }
}
