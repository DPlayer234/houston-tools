//! Provides data structures for ship equipment.

use serde::{Deserialize, Serialize};
use small_fixed_array::{FixedArray, FixedString};

use crate::ship::*;
use crate::skill::*;
use crate::{Faction, define_data_enum};

/// Represents a piece of equipment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equip {
    /// The equipment's ID. This differs per upgrade step.
    pub equip_id: u32,
    /// The equipment's display name.
    pub name: FixedString,
    /// The equipment's description.
    ///
    /// This is not the skill description. Instead, it is the description shown
    /// when attempting to buy equipment from a shop. It is never seen for most
    /// gear, but often still contains flavor text.
    pub description: FixedString,
    /// The kind of equipment, determining whether it is allowed in a ship's
    /// slots.
    pub kind: EquipKind,
    /// The equipment's rarity and star rating.
    pub rarity: EquipRarity,
    /// The manufacturer faction.
    pub faction: Faction,
    /// The weapons this equipment carries.
    ///
    /// This will usually just hold a single element.
    /// The most common case where this doesn't hold is aircraft with intercept;
    /// the strike and intercept versions are different weapons.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub weapons: FixedArray<Weapon>,
    /// Skills this equipment activates when equipped.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub skills: FixedArray<Skill>,
    /// The stat bonuses provided when equipped.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub stat_bonuses: FixedArray<EquipStatBonus>,
    /// Hull types that this equipment cannot be equipped on, even if the
    /// [`Equip::kind`] would allow it.
    ///
    /// Data on "allowed hull types" is excluded since it's purely informative,
    /// and not accurately at that.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub hull_disallowed: FixedArray<HullType>,
}

/// A weapon that is part of [`Equip`] or [`Skill`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weapon {
    /// The weapon's ID. This differs per upgrade step/skill level.
    pub weapon_id: u32,
    /// The weapon's display name, if any.
    ///
    /// This is the label for a weapon on an aircraft.
    pub name: Option<FixedString>,
    /// The base reload time.
    ///
    /// This component is affected by the ship's RLD stat.
    /// The value stored here is the reload time at 100 RLD.
    pub reload_time: f64,
    /// A fixed delay between reloads.
    pub fixed_delay: f64,
    /// The kind of weapon.
    pub kind: WeaponKind,
    /// The actual data for the weapon.
    pub data: WeaponData,
}

/// A bullet barrage pattern for a [`Weapon`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Barrage {
    /// The damage per bullet.
    pub damage: f64,
    /// The coefficient. This is a straight up damage multiplier.
    pub coefficient: f64,
    /// How much of the [`Barrage::scaling_stat`] is considered for the damage.
    ///
    /// F.e. `1.0` would mean you use 100% of the scaling stat for damage
    /// calculation, whereas `0.8` would mean you only use 80% of it.
    /// This leads to slightly different results than a damage multiplier.
    pub scaling: f64,
    /// The stat this barrage's damage scales with.
    pub scaling_stat: StatKind,
    /// How far the bullets will launch.
    pub range: f64,
    /// The potential firing and lock-on angle.
    pub firing_angle: f64,
    /// The time it takes for the salvo to fire. This is another fixed delay
    /// between reloads.
    pub salvo_time: f64,
    /// The bullets fired by this barrage.
    pub bullets: FixedArray<Bullet>,
}

/// Bullet information for a [`Barrage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bullet {
    /// The bullet's ID.
    pub bullet_id: u32,

    // From barrage template:
    /// The amount of bullets fired by this component.
    pub amount: u32,

    // From bullet template:
    /// The kind of bullet fired.
    pub kind: BulletKind,
    /// The kind of ammo this bullet uses.
    pub ammo: AmmoKind,
    /// How often the bullets can pierce enemies before it disappears.
    pub pierce: u32,
    /// The velocity these bullets are fired at.
    pub velocity: f64,
    /// The armor modifiers.
    pub modifiers: ArmorModifiers,
    /// Additional flags for the bullets.
    pub flags: BulletFlags,

    /// Buffs caused by the bullet hit.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub attach_buff: FixedArray<BuffInfo>,

    /// Extra data depending on the bullet type.
    #[serde(default, skip_serializing_if = "BulletExtra::is_none")]
    pub extra: BulletExtra,
}

/// Additional bullet data.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BulletExtra {
    /// No extra data.
    #[default]
    None,
    /// The bullet has hit spread and AOE.
    Spread(BulletSpread),
    /// The bullet is a beam attack.
    Beam(BulletBeam),
}

/// How far a bullet's hit spread and AOE is. Only applicable to main gun fire
/// and bombs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletSpread {
    /// Horizontal spread.
    pub spread_x: f64,
    /// Vertical spread.
    pub spread_y: f64,
    /// The range for the splash damage.
    pub hit_range: f64,
}

/// Additional information about a beam.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletBeam {
    /// The total duration of the beam.
    pub duration: f64,
    /// The delay between damage ticks.
    pub tick_delay: f64,
}

/// Aircraft data for a [`Weapon`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aircraft {
    /// The aircraft's ID. This differs per upgrade step/skill level.
    pub aircraft_id: u32,
    /// The amount of aircraft launched.
    pub amount: u32,
    /// The aircrafts' flight speed.
    pub speed: f64,
    /// The aircrafts' individual health.
    pub health: ShipStat,
    /// How often each aircraft is allowed to dodge attacks.
    pub dodge_limit: u32,
    /// The aircraft-mounted weapons.
    pub weapons: FixedArray<Weapon>,
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
pub struct ArmorModifiers(
    /// Modifier to Light armor.
    pub f64,
    /// Modifier to Medium armor.
    pub f64,
    /// Modifier to Heavy armor.
    pub f64,
);

/// Bonus stats gained by equipping the associated equipment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipStatBonus {
    /// The stat increased.
    pub stat_kind: StatKind,
    /// How much the stat is increased by.
    pub amount: f64,
}

/// Represents an Augment Module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Augment {
    /// The augment's ID.
    pub augment_id: u32,
    /// The augment's display name.
    pub name: FixedString,
    /// The augment's rarity and star rating.
    pub rarity: AugmentRarity,
    /// The stat bonuses provided by the augment.
    pub stat_bonuses: FixedArray<AugmentStatBonus>,
    /// Who can equip this augment.
    pub usability: AugmentUsability,
    /// The augment's primary effect skill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<Skill>,
    /// The augment's skill upgrade.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub skill_upgrades: FixedArray<AugmentSkillUpgrade>,
}

/// Represents who an Augment Module can be used on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AugmentUsability {
    /// Only certain hull types are allowed.
    HullTypes(FixedArray<HullType>),
    /// Only a certain unique ship is allowed.
    UniqueShipId(u32),
}

/// Bonus stats gained by equipping the associated augment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentStatBonus {
    /// The stat increased.
    pub stat_kind: StatKind,
    /// How much the stat is increased by at minimum.
    pub amount: f64,
    /// The maximum additional random component.
    pub random: f64,
}

/// A skill upgraded by an augment module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentSkillUpgrade {
    /// The ID of the original skill to replace.
    pub original_id: u32,
    /// The replacement skill.
    pub skill: Skill,
}

bitflags::bitflags! {
    /// Additional flags for a bullet.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct BulletFlags: u8 {
        /// Ignores bullet shields.
        const IGNORE_SHIELD = 1 << 0;
        /// Ignores surface ships.
        const IGNORE_SURFACE = 1 << 1;
        /// Ignores diving ships.
        const IGNORE_DIVE = 1 << 2;
    }
}

define_data_enum! {
    /// The possible kinds of equipment.
    pub enum EquipKind for EquipKindData {
        /// A friendly name for the equipment kind.
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
    /// The kind a weapon is classified as.
    ///
    /// The values here can be reasonable to entirely unintuitive.
    /// You might be looking for [`EquipKind`] instead.
    pub enum WeaponKind for WeaponKindData {
        /// A friendly name for the weapon kind.
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

impl BulletExtra {
    /// Whether this bullet extra is empty.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl BulletFlags {
    /// Filters to the flags that are relevant for the dive filter,
    /// i.e. which targets the bullet _can't_ hit.
    #[must_use]
    pub fn dive_filter(self) -> Self {
        self & (Self::IGNORE_SURFACE | Self::IGNORE_DIVE)
    }
}

impl AugmentUsability {
    /// If restricted by hull types, gets the hull types. Otherwise, returns
    /// [`None`].
    pub fn hull_types(&self) -> Option<&[HullType]> {
        match self {
            Self::HullTypes(h) => Some(h.as_slice()),
            _ => None,
        }
    }

    /// If restricted to a unique ship, gets its ID. Otherwise, returns
    /// [`None`].
    pub fn unique_ship_id(&self) -> Option<u32> {
        match self {
            Self::UniqueShipId(i) => Some(*i),
            _ => None,
        }
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
        *self.modifier_mut(armor_kind) = value;
        self
    }

    #[inline]
    fn modifier_mut(&mut self, armor_kind: ShipArmor) -> &mut f64 {
        match armor_kind {
            ShipArmor::Light => &mut self.0,
            ShipArmor::Medium => &mut self.1,
            ShipArmor::Heavy => &mut self.2,
        }
    }
}

impl From<[f64; 3]> for ArmorModifiers {
    fn from([l, m, h]: [f64; 3]) -> Self {
        Self(l, m, h)
    }
}

impl From<(f64, f64, f64)> for ArmorModifiers {
    fn from((l, m, h): (f64, f64, f64)) -> Self {
        Self(l, m, h)
    }
}
