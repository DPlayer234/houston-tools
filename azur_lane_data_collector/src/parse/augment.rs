use std::num::NonZeroU32;
use mlua::prelude::*;
use azur_lane::equip::*;

use crate::context;
use crate::convert_al;
use crate::parse;
use crate::model::*;

/// Construct augment data from this set.
pub fn load_augment(lua: &Lua, set: &AugmentSet) -> LuaResult<Augment> {
    /// Reads a value from the statistics; target-typed.
    macro_rules! read {
        ($field:expr) => {
            context!(set.statistics.get($field); "{} of augment with id {}", $field, set.id)?
        };
    }

    /// Reads a [`azur_lane::ship::StatKind`] from the statistics.
    macro_rules! read_stat {
        ($field:expr) => {{
            let temp: String = read!($field);
            convert_al::to_stat_kind(&temp)
        }};
    }

    // The effect is the first skill on the augment, f.e. a slash attack.
    // This field is always present, but 0 indicates that no effect is used.
    let effect: u32 = read!("effect_id");
    let effect = match effect {
        0 => None,
        _ => Some(parse::skill::load_skill(lua, effect)?)
    };

    // For unique augments, there is a list of skill upgrades.
    // In practice, this is never more than one.
    let skill_upgrade: Vec<LuaTable> = read!("skill_upgrade");
    let skill_upgrade = match skill_upgrade.into_iter().next() {
        Some(skill_upgrade) => {
            let skill_id: u32 = context!(skill_upgrade.get(2); "skill_upgrade id for augment {}", set.id)?;
            Some(parse::skill::load_skill(lua, skill_id)?)
        }
        None => None,
    };

    // ID of the only ship group this can be equipped to, if unique.
    // As with the effect, always present but 0 if not used.
    let unique_ship_id: u32 = read!("unique");
    let unique_ship_id = NonZeroU32::new(unique_ship_id);

    Ok(Augment {
        augment_id: set.id,
        name: From::<String>::from(read!("name")),
        stat_bonuses: vec![
            AugmentStatBonus {
                stat_kind: read_stat!("attribute_1"),
                amount: read!("value_1"),
                random: read!("value_1_random")
            },
            AugmentStatBonus {
                stat_kind: read_stat!("attribute_2"),
                amount: read!("value_2"),
                random: read!("value_2_random")
            }
        ],
        allowed: {
            let allowed: Vec<u32> = read!("usability");
            allowed.into_iter().map(convert_al::to_hull_type).collect()
        },
        effect,
        unique_ship_id,
        skill_upgrade,
    })
}

