use std::sync::Arc;
use serde::*;

use crate::define_data_enum;
use crate::ship::*;
use super::Faction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equip {
    pub kind: EquipKind,
    pub faction: Faction,
    pub hull_allowed: Arc<[HullType]>,
    pub hull_disallowed: Arc<[HullType]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weapon: Option<Weapon>,
    pub stat_bonuses: Arc<[EquipStatBonus]>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weapon {
    // From weapon property:
    pub weapon_id: u32,
    pub reload_time: f32,
    pub fixed_delay: f32,
    pub damage: f32,
    pub coefficient: f32,
    pub scaling: f32,
    pub scaling_stat: StatKind,
    pub range: f32,
    pub firing_angle: f32,

    // From bullet template:
    pub pierce: u32,
    pub kind: BulletKind,
    pub velocity: f32,
    pub modifiers: ArmorModifiers,
    
    // From barrage template:
    pub amount: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmorModifiers(pub f32, pub f32, pub f32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipStatBonus {
    pub stat_kind: StatKind,
    pub amount: f32
}

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
    pub enum BulletKind for BulletKindData {
        pub name: &'static str;

        Normal("Normal"),
        AP("AP"),
        HE("HE"),
        Torpedo("Torpedo"),
        Bomb("Bomb"),
        SAP("SAP"),
        AntiSub("Anti-Sub")
    }
}

impl ArmorModifiers {
    pub fn get_modifier(&self, armor_kind: ShipArmor) -> f32 {
        match armor_kind {
            ShipArmor::Light => self.0,
            ShipArmor::Medium => self.1,
            ShipArmor::Heavy => self.2,
        }
    }
}
