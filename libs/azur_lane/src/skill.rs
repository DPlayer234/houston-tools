//! Provides a subset of data for ship/equipment skills.

use serde::{Deserialize, Serialize};
use small_fixed_array::{FixedArray, FixedString};

use crate::data_def::{define_data_enum, is_default};
use crate::equip::Weapon;

/// Represents a single skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// The skill's ID.
    ///
    /// This is named `buff_id` since skills, as shown in-game, actually refer
    /// to buffs. A buff stays attached, while a skill is an instant effect.
    pub buff_id: u32,
    /// The skill's name.
    pub name: FixedString,
    /// The skill's description, with placeholders already replaced.
    pub description: FixedString,
    /// The category of this skill.
    pub category: SkillCategory,
    /// Barrages this skill can fire.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub barrages: FixedArray<SkillBarrage>,
    /// Weapons this skill may add to the ship.
    #[serde(default, skip_serializing_if = "FixedArray::is_empty")]
    pub new_weapons: FixedArray<BuffWeapon>,
}

/// Represents a skill barrage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBarrage {
    /// The ID of the skill that fires this barrage.
    pub skill_id: u32,
    /// The attacks within this barrage.
    pub attacks: FixedArray<SkillAttack>,
}

/// Represents a skill barrage's attack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAttack {
    /// The target this attack fires at.
    pub target: SkillAttackTarget,
    /// The weapon fired by this attack.
    pub weapon: Weapon,
}

/// Represents a buff's bonus weapon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuffWeapon {
    /// The weapon to be attached.
    pub weapon: Weapon,
    /// How long this weapon lasts.
    ///
    /// [`None`] means it will last indefinitely.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// The weapon that gets replaced.
    ///
    /// If [`None`], no weapons are replaced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace: Option<BuffWeaponReplace>,
}

/// The filter for the weapon(s) to replace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuffWeaponReplace {
    /// Indicates that this weapon has to be a replacement, but the mode isn't
    /// recognized.
    Unknown,
    /// Replaces the weapons with the specified ID.
    Id(u32),
    /// Replaces the weapons with any of the specified labels.
    Label(FixedArray<FixedString>),
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
        Fixed("Fixed", "Fixed")
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
    /// The ID of the inflicted buff.
    pub buff_id: u32,
    /// The chance to inflict it. Expected to be in range `[0.0 .. 1.0]`.
    pub probability: f64,
    /// The buff level.
    #[serde(default, skip_serializing_if = "is_default")]
    pub level: u32,
}
