//! Provides data structures for ship equipment.

use serde::*;

use crate::define_data_enum;
use crate::ship::*;
use crate::skill::*;
use super::Faction;

/// Represents a piece of equipment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equip {
    pub name: String,
    pub kind: EquipKind,
    pub faction: Faction,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub hull_allowed: Vec<HullType>,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub hull_disallowed: Vec<HullType>,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub weapons: Vec<Weapon>,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub stat_bonuses: Vec<EquipStatBonus>
}

/// A weapon that is part of [`Equip`] or [`Skill`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weapon {
    pub weapon_id: u32,
    pub reload_time: f32,
    pub fixed_delay: f32,
    pub data: WeaponData
}

/// A bullet barrage pattern for a [`Weapon`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Barrage {
    pub damage: f32,
    pub coefficient: f32,
    pub scaling: f32,
    pub scaling_stat: StatKind,
    pub range: f32,
    pub firing_angle: f32,
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
    pub velocity: f32,
    pub modifiers: ArmorModifiers,

    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub attach_buff: Vec<BuffInfo>,

    /// How far the hit spread is. Only applicable to main gun fire and bombs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spread: Option<BulletSpread>,
}

/// How far a bullet's hit spread is. Only applicable to main gun fire and bombs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletSpread {
    pub spread_x: f32,
    pub spread_y: f32,
    pub hit_range: f32,
}

/// Aircraft data for a [`Weapon`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aircraft {
    pub aircraft_id: u32,
    pub amount: u32,
    pub speed: f32,
    pub weapons: Vec<Weapon>
}

/// The possible data a [`Weapon`] can hold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WeaponData {
    /// The weapon fires bullets as a [`Barrage`].
    Bullets(Barrage),
    /// The weapon launches an [`Aircraft`].
    Aircraft(Aircraft),
}

/// Armor modifiers to apply to the damage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArmorModifiers(pub f32, pub f32, pub f32);

/// Bonus stats gained by equipping the associated equipment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipStatBonus {
    pub stat_kind: StatKind,
    pub amount: f32
}

/// Represents an Augment Module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Augment {
    pub augment_id: u32,
    pub name: String,
    pub stat_bonuses: Vec<AugmentStatBonus>,
    pub allowed: Vec<HullType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<Skill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unique_ship_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_upgrade: Option<Skill>
}

/// Bonus stats gained by equipping the associated augment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentStatBonus {
    pub stat_kind: StatKind,
    pub amount: f32,
    pub random: f32
}

/// The possible kinds of equipment.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EquipKind {
    DestroyerGun,
    LightCruiserGun,
    HeavyCruiserGun,
    LargeCruiserGun,
    BattleshipGun,
    SurfaceTorpedo,
    SubmarineTorpedo,
    AntiAirGun,
    FuzeAntiAirGun,
    Fighter,
    DiveBomber,
    TorpedoBomber,
    SeaPlane,
    AntiSubWeapon,
    AntiSubAircraft,
    Helicopter,
    Missile,
    Cargo,
    Auxiliary,
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
        Unknown5("5", "?"),
        Bomb("Bomb", "Bomb"),
        SAP("SAP", "SAP"),
        Unknown8("8", "?"),
        Unknown9("9", "?")
    }
}

impl ArmorModifiers {
    /// Gets the modifier for a specific kind of armor.
    #[must_use]
    pub fn get_modifier(&self, armor_kind: ShipArmor) -> f32 {
        match armor_kind {
            ShipArmor::Light => self.0,
            ShipArmor::Medium => self.1,
            ShipArmor::Heavy => self.2,
        }
    }
}

impl From<[f32; 3]> for ArmorModifiers {
    fn from(value: [f32; 3]) -> Self {
        ArmorModifiers(value[0], value[1], value[2])
    }
}

impl From<(f32, f32, f32)> for ArmorModifiers {
    fn from(value: (f32, f32, f32)) -> Self {
        ArmorModifiers(value.0, value.1, value.2)
    }
}
