use azur_lane::ship::*;
use mlua::prelude::*;

use crate::intl_util::IterExt as _;
use crate::{context, convert_al};

pub fn load_ship_tech(_lua: &Lua, id: u32, set: LuaTable) -> LuaResult<FleetTechInfo> {
    macro_rules! read {
        ($field:expr) => {
            set.get($field)
                .with_context(context!("{} of fleet tech with id {id}", $field))?
        };
    }

    macro_rules! read_stat {
        ($field:expr) => {
            convert_al::num_to_stat_kind(read!($field))
        };
    }

    macro_rules! read_hull_types {
        ($field:expr) => {{
            let temp: Vec<u32> = read!($field);
            temp.into_iter()
                .map(convert_al::to_hull_type)
                .collect_fixed_array()
        }};
    }

    Ok(FleetTechInfo {
        class: read!("class"),
        pt_get: read!("pt_get"),
        pt_level: read!("pt_level"),
        pt_limit_break: read!("pt_upgrage"), /* sic */
        stats_get: FleetTechStatBonus {
            hull_types: read_hull_types!("add_get_shiptype"),
            stat: read_stat!("add_get_attr"),
            amount: read!("add_get_value"),
        },
        stats_level: FleetTechStatBonus {
            hull_types: read_hull_types!("add_level_shiptype"),
            stat: read_stat!("add_level_attr"),
            amount: read!("add_level_value"),
        },
    })
}
