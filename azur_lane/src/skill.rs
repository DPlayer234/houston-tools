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
    #[serde(default = "make_empty_arc", skip_serializing_if = "is_empty_arc")]
    pub weapons: Arc<[SkillAttack]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAttack {
    pub weapon: Weapon,
    pub target: SkillAttackTarget,
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
