//! Provides a subset of data for ship/equipment skills.

use serde::{Serialize, Deserialize};

use crate::define_data_enum;
use crate::equip::Weapon;

/// Represents a single skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub buff_id: u32,
    pub name: String,
    pub description: String,
    pub category: SkillCategory,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub barrages: Vec<SkillBarrage>,
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    pub new_weapons: Vec<Weapon>,
}

/// Represents a skill barrage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBarrage {
    pub skill_id: u32,
    pub attacks: Vec<SkillAttack>,
}

/// Represents a skill barrage's attack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAttack {
    pub target: SkillAttackTarget,
    pub weapon: Weapon,
}

define_data_enum! {
    /// How a barrage attack chooses its target.
    pub enum SkillAttackTarget for SkillAttackTargetData {
        /// The friendly display name for the targeting.
        pub friendly_name: &'static str,
        /// A short-hand name.
        pub short_name: &'static str;

        Random("Random", "Rand."),
        PriorityTarget("Priority Target", "Prio."),
        Nearest("Nearest", "Near."),
        Farthest("Farthest", "Far."),
        Fixed("Fixed", "Fix.")
    }
}

define_data_enum! {
    /// The category of the skill, or its "color".
    pub enum SkillCategory for SkillCategoryData {
        /// A friendly display name for the category.
        pub friendly_name: &'static str,
        /// A color matching the category.
        pub color_rgb: u32,
        /// An emoji for the category.
        pub emoji: char;

        Offense("Offense", 0xDD2E44, '🟥'),
        Defense("Defense", 0x55ACEE, '🟦'),
        Support("Support", 0xFDCB58, '🟨')
    }
}

/// Represents basic information about a buff, to be extended later if needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuffInfo {
    pub buff_id: u32,
    pub probability: f64,
    #[serde(default, skip_serializing_if = "crate::data_def::is_default")]
    pub level: u32,
}
