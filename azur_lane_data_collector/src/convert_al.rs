use azur_lane::*;
use azur_lane::equip::*;
use azur_lane::ship::*;
use azur_lane::skill::*;

pub fn to_faction(num: u32) -> Faction {
    match num {
        0 => Faction::Universal,
        1 => Faction::EagleUnion,
        2 => Faction::RoyalNavy,
        3 => Faction::SakuraEmpire,
        4 => Faction::IronBlood,
        5 => Faction::DragonEmpery,
        6 => Faction::SardegnaEmpire,
        7 => Faction::NorthernParliament,
        8 => Faction::IrisLibre,
        9 => Faction::VichyaDominion,
        96 => Faction::Tempesta,
        97 => Faction::META,
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
        _ => Faction::Unknown
    }
}

pub fn to_rarity(num: u32) -> ShipRarity {
    match num {
        1 | 2 => ShipRarity::N,
        3 => ShipRarity::R,
        4 => ShipRarity::E,
        5 => ShipRarity::SR,
        6 => ShipRarity::UR,
        _ => ShipRarity::N
    }
}

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
        10 => HullType::AviationBattleship,
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
        _ => HullType::Unknown
    }
}

pub fn to_armor_type(num: u32) -> ShipArmor {
    match num {
        1 => ShipArmor::Light,
        2 => ShipArmor::Medium,
        3 => ShipArmor::Heavy,
        _ => ShipArmor::Light
    }
}

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
        8 => EquipKind::DiveBomber,
        9 => EquipKind::TorpedoBomber,
        12 => EquipKind::SeaPlane,
        14 => EquipKind::AntiSubWeapon,
        15 => EquipKind::AntiSubAircraft,
        17 => EquipKind::Helicopter,
        20 => EquipKind::Missile,
        18 => EquipKind::Cargo,
        10 | _ => EquipKind::Auxiliary,
    }
}

pub fn to_stat_kind(num: u32) -> StatKind {
    match num {
        1 => StatKind::HP,
        6 => StatKind::RLD,
        2 => StatKind::FP,
        3 => StatKind::TRP,
        9 => StatKind::EVA,
        4 => StatKind::AA,
        5 => StatKind::AVI,
        8 => StatKind::ACC,
        12 => StatKind::ASW,
        10 => StatKind::SPD,
        11 => StatKind::LCK,
        _ => StatKind::HP
    }
}

pub fn to_barrage_target(text: &str) -> SkillAttackTarget {
    match text {
        "TargetHarmRandom" => SkillAttackTarget::Random,
        "TargetHarmRandomByWeight" => SkillAttackTarget::PriorityTarget,
        "TargetHarmNearest" => SkillAttackTarget::Nearest,
        "TargetHarmFarthest" => SkillAttackTarget::Farthest,
        "TargetNil" | _ => SkillAttackTarget::Fixed
    }
}
