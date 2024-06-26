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

    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub attach_buff: Vec<BuffInfo>,

    /// How far the hit spread is. Only applicable to main gun fire and bombs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spread: Option<BulletSpread>,
}

/// How far a bullet's hit spread is. Only applicable to main gun fire and bombs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletSpread {
    pub spread_x: f64,
    pub spread_y: f64,
    pub hit_range: f64,
}

/// Aircraft data for a [`Weapon`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aircraft {
    pub aircraft_id: u32,
    pub amount: u32,
    pub speed: f64,
    pub weapons: Vec<Weapon>
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
    pub amount: f64,
    pub random: f64
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
        SubGun("Secondary Gun"),
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

impl ArmorModifiers {
    /// Gets the modifier for a specific kind of armor.
    #[must_use]
    pub fn get_modifier(&self, armor_kind: ShipArmor) -> f64 {
        match armor_kind {
            ShipArmor::Light => self.0,
            ShipArmor::Medium => self.1,
            ShipArmor::Heavy => self.2,
        }
    }

    /// Sets the modifier for a specific kind of armor.
    #[must_use]
    pub fn set_modifier(mut self, armor_kind: ShipArmor, value: f64) -> Self {
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
