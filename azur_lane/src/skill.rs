use std::sync::Arc;
use serde::*;

use crate::define_data_enum;
use crate::equip::Weapon;
use crate::data_def::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub buff_id: u32,
    pub name: Arc<str>,
    pub description: Arc<str>,
    pub category: SkillCategory,
    #[serde(default = "make_empty_arc", skip_serializing_if = "is_empty_arc")]
    pub barrages: Arc<[SkillBarrage]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBarrage {
    pub skill_id: u32,
    pub attacks: Arc<[SkillAttack]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAttack {
    pub target: SkillAttackTarget,
    pub weapon: Weapon,
}

define_data_enum! {
    pub enum SkillAttackTarget for BarrageTargetData {
        pub friendly_name: &'static str;

        Random("Random"),
        PriorityTarget("Priority Target"),
        Nearest("Nearest"),
        Farthest("Farthest"),
        Fixed("Fixed")
    }
}

define_data_enum! {
    pub enum SkillCategory for SkillCategoryData {
        pub friendly_name: &'static str,
        pub color_rgb: u32,
        pub emoji: char;

        Offense("Offense", 0xDD2E44, '🟥'),
        Defense("Defense", 0x55ACEE, '🟦'),
        Support("Support", 0xFDCB58, '🟨')
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuffInfo {
    pub buff_id: u32,
    pub probability: f32,
    #[serde(default, skip_serializing_if = "is_default")]
    pub level: u32,
}
