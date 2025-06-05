//! Provides functions to convert primitives to AL model values.

// allowed to be explicit about the intended source value
#![expect(clippy::wildcard_in_or_patterns)]

use azur_lane::Faction;
use azur_lane::equip::*;
use azur_lane::ship::*;
use azur_lane::skill::*;

/// Converts an ID to a faction.
pub fn to_faction(num: u32) -> Faction {
    match num {
        0 | 98 => Faction::Universal, // 98 is technically "Bulin"
        1 => Faction::EagleUnion,
        2 => Faction::RoyalNavy,
        3 => Faction::SakuraEmpire,
        4 => Faction::IronBlood,
        5 => Faction::DragonEmpery,
        6 => Faction::SardegnaEmpire,
        7 => Faction::NorthernParliament,
        8 => Faction::IrisLibre,
        9 => Faction::VichyaDominion,
        10 => Faction::IrisOrthodoxy,
        11 => Faction::KingdomOfTulipa,
        96 => Faction::Tempesta,
        97 => Faction::Meta,
        99 => Faction::Siren,
        101 => Faction::CollabNeptunia,
        102 => Faction::CollabBilibili,
        103 => Faction::CollabUtawarerumono,
        104 => Faction::CollabKizunaAI,
        105 => Faction::CollabHololive,
        106 => Faction::CollabVenusVacation,
        107 => Faction::CollabIdolmaster,
        108 => Faction::CollabSSSS,
        109 => Faction::CollabAtelierRyza,
        110 => Faction::CollabSenranKagura,
        111 => Faction::CollabToLoveRu,
        112 => Faction::CollabBlackRockShooter,
        _ => Faction::Unknown,
    }
}

/// Converts an ID to a rarity.
pub fn to_rarity(num: u32) -> ShipRarity {
    match num {
        1 | 2 => ShipRarity::N,
        3 => ShipRarity::R,
        4 => ShipRarity::E,
        5 => ShipRarity::SR,
        6 => ShipRarity::UR,
        _ => ShipRarity::N,
    }
}

/// Converts an ID to a hull type.
pub fn to_hull_type(num: u32) -> HullType {
    match num {
        1 => HullType::Destroyer,
        2 => HullType::LightCruiser,
        3 => HullType::HeavyCruiser,
        4 => HullType::Battlecruiser,
        5 => HullType::Battleship,
        6 => HullType::LightCarrier,
        7 => HullType::AircraftCarrier,
        8 => HullType::Submarine,
        // 9 => HullType::AviationCruiser,
        10 => HullType::AviationBattleship,
        // 11 => HullType::TorpedoCruiser,
        12 => HullType::RepairShip,
        13 => HullType::Monitor,
        17 => HullType::AviationSubmarine,
        18 => HullType::LargeCruiser,
        19 => HullType::MunitionShip,
        20 => HullType::MissileDestroyerV,
        21 => HullType::MissileDestroyerM,
        22 => HullType::FrigateS,
        23 => HullType::FrigateV,
        24 => HullType::FrigateM,
        _ => HullType::Unknown,
    }
}

/// Converts an ID to a known hull type.
///
/// Returns [`None`] if it is unknown.
pub fn to_known_hull_type(num: u32) -> Option<HullType> {
    let hull_type = to_hull_type(num);
    (hull_type != HullType::Unknown).then_some(hull_type)
}

/// Converts an ID to an armor type.
pub fn to_armor_type(num: u32) -> ShipArmor {
    match num {
        1 => ShipArmor::Light,
        2 => ShipArmor::Medium,
        3 => ShipArmor::Heavy,
        _ => ShipArmor::Light,
    }
}

/// Converts an ID to an equipment kind.
pub fn to_equip_type(num: u32) -> EquipKind {
    match num {
        1 => EquipKind::DestroyerGun,
        2 => EquipKind::LightCruiserGun,
        3 => EquipKind::HeavyCruiserGun,
        11 => EquipKind::LargeCruiserGun,
        4 => EquipKind::BattleshipGun,
        5 => EquipKind::SurfaceTorpedo,
        13 => EquipKind::SubmarineTorpedo,
        6 => EquipKind::AntiAirGun,
        21 => EquipKind::FuzeAntiAirGun,
        7 => EquipKind::Fighter,
        9 => EquipKind::DiveBomber,
        8 => EquipKind::TorpedoBomber,
        12 => EquipKind::SeaPlane,
        14 => EquipKind::AntiSubWeapon,
        15 => EquipKind::AntiSubAircraft,
        17 => EquipKind::Helicopter,
        20 => EquipKind::Missile,
        18 => EquipKind::Cargo,
        10 | _ => EquipKind::Auxiliary,
    }
}

/// Converts a number to an equip rarity.
pub fn to_equip_rarity(num: u32) -> EquipRarity {
    match num {
        ..=1 => EquipRarity::N1,
        2 => EquipRarity::N2,
        3 => EquipRarity::R,
        4 => EquipRarity::E,
        5 => EquipRarity::SR,
        _ => EquipRarity::UR,
    }
}

/// Converts a number to an augment rarity.
pub fn to_augment_rarity(num: u32) -> AugmentRarity {
    match num {
        ..=2 => AugmentRarity::R,
        3 => AugmentRarity::E,
        _ => AugmentRarity::SR,
    }
}

/// Converts a name to a stat kind.
pub fn to_stat_kind(stat: &str) -> StatKind {
    match stat {
        "durability" => StatKind::HP,
        "cannon" => StatKind::FP,
        "torpedo" => StatKind::TRP,
        "antiaircraft" => StatKind::AA,
        "air" => StatKind::AVI,
        "reload" => StatKind::RLD,
        "hit" => StatKind::ACC,
        "dodge" => StatKind::EVA,
        "speed" => StatKind::SPD,
        "luck" => StatKind::LCK,
        "antisub" => StatKind::ASW,
        _ => StatKind::EVA,
    }
}

/// Converts a weapon attack attribute number to a stat kind.
pub fn weapon_attack_attr_to_stat_kind(num: u32) -> StatKind {
    match num {
        1 => StatKind::FP,
        2 => StatKind::TRP,
        3 => StatKind::AA,
        4 => StatKind::AVI,
        5 => StatKind::ASW,
        _ => StatKind::LCK,
    }
}

/// Converts a name to a skill target.
pub fn to_skill_target(text: &str) -> SkillAttackTarget {
    match text {
        "TargetHarmRandom" => SkillAttackTarget::Random,
        "TargetHarmRandomByWeight" => SkillAttackTarget::PriorityTarget,
        "TargetHarmNearest" => SkillAttackTarget::Nearest,
        "TargetHarmFarthest" => SkillAttackTarget::Farthest,
        "TargetNil" | _ => SkillAttackTarget::Fixed,
    }
}

/// Converts an ID to a skill category.
pub fn to_skill_category(num: u32) -> SkillCategory {
    match num {
        1 => SkillCategory::Offense,
        2 => SkillCategory::Defense,
        _ => SkillCategory::Support,
    }
}

/// Converts an ID to a bullet kind.
pub fn to_bullet_kind(num: u32) -> BulletKind {
    match num {
        1 => BulletKind::Cannon,
        2 => BulletKind::Bomb,
        3 => BulletKind::Torpedo,
        4 => BulletKind::Direct,
        5 => BulletKind::Shrapnel,
        6 => BulletKind::AntiAir,
        7 => BulletKind::AntiSea,
        9 => BulletKind::Effect,
        10 => BulletKind::Beam,
        11 => BulletKind::GBullet,
        12 => BulletKind::EletricArc,
        13 => BulletKind::Missile,
        14 => BulletKind::SpaceLaser,
        15 => BulletKind::Scale,
        16 => BulletKind::TriggerBomb,
        17 => BulletKind::AAMissile,
        _ => BulletKind::Cannon,
    }
}

/// Converts an ID to a weapon kind.
pub fn to_weapon_kind(num: u32) -> WeaponKind {
    match num {
        1 => WeaponKind::MainGun,
        2 => WeaponKind::SubGun,
        3 => WeaponKind::Torpedo,
        4 => WeaponKind::AirToAir,
        5 => WeaponKind::Armor,
        6 => WeaponKind::Engine,
        7 => WeaponKind::Radar,
        10 => WeaponKind::StrikeAircraft,
        11 => WeaponKind::InterceptAircraft,
        12 => WeaponKind::Crew,
        13 => WeaponKind::Charge,
        14 => WeaponKind::Special,
        15 => WeaponKind::MegaCharge,
        16 => WeaponKind::ManualTorpedo,
        17 => WeaponKind::AntiSub,
        18 => WeaponKind::HammerHead,
        19 => WeaponKind::BomberPreCastAlert,
        20 => WeaponKind::MultiLock,
        21 => WeaponKind::ManualSub,
        22 => WeaponKind::AntiAir,
        23 => WeaponKind::Bracketing,
        24 => WeaponKind::Beam,
        25 => WeaponKind::DepthCharge,
        26 => WeaponKind::AntiAirRepeater,
        27 => WeaponKind::DisposableTorpedo,
        28 => WeaponKind::SpaceLaser,
        29 => WeaponKind::Missile,
        30 => WeaponKind::AntiAirFuze,
        31 => WeaponKind::ManualMissile,
        32 => WeaponKind::AutoMissile,
        33 => WeaponKind::Meteor,
        _ => WeaponKind::Unknown,
    }
}

/// Converts an ID to an ammo kind.
pub fn to_ammo_kind(num: u32) -> AmmoKind {
    match num {
        1 => AmmoKind::Normal,
        2 => AmmoKind::AP,
        3 => AmmoKind::HE,
        4 => AmmoKind::Torpedo,
        5 => AmmoKind::AirToAir,
        6 => AmmoKind::Bomb,
        7 => AmmoKind::SAP,
        8 => AmmoKind::Unknown8,
        9 => AmmoKind::Unknown9,
        _ => AmmoKind::Normal,
    }
}

/// Converts to a dive filter.
pub fn to_dive_filter(num: u32) -> BulletFlags {
    match num {
        1 => BulletFlags::IGNORE_SURFACE,
        2 => BulletFlags::IGNORE_DIVE,
        _ => BulletFlags::empty(),
    }
}
