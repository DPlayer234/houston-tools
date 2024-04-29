use serde::*;

use crate::define_data_enum;
use crate::equip::Weapon;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub buff_id: u32,
    pub name: String,
    pub description: String,
    pub category: SkillCategory,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub barrages: Vec<SkillBarrage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBarrage {
    pub skill_id: u32,
    pub attacks: Vec<SkillAttack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAttack {
    pub target: SkillAttackTarget,
    pub weapon: Weapon,
}

define_data_enum! {
    pub enum SkillAttackTarget for SkillAttackTargetData {
        pub friendly_name: &'static str,
        pub short_name: &'static str;

        Random("Random", "Rand."),
        PriorityTarget("Priority Target", "Prio."),
        Nearest("Nearest", "Near."),
        Farthest("Farthest", "Far."),
        Fixed("Fixed", "Fix.")
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
    #[serde(default, skip_serializing_if = "crate::data_def::is_default")]
    pub level: u32,
}
