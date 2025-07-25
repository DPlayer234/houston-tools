use azur_lane::ship::*;
use mlua::prelude::*;
use small_fixed_array::{FixedArray, TruncatingInto as _};

use crate::intl_util::{IterExt as _, TryIterExt as _};
use crate::model::*;
use crate::{context, convert_al, enhance, parse};

/// Constructs ship data from this set.
pub fn load_ship_data(lua: &Lua, set: &ShipSet<'_>) -> LuaResult<ShipData> {
    /// Reads a single value; target-typed.
    macro_rules! read {
        ($table:expr, $field:expr) => {
            $table
                .get($field)
                .with_context(context!("{} of ship with id {}", $field, set.id))?
        };
    }

    let attrs: LuaTable = read!(set.statistics, "attrs");
    let attrs_growth: LuaTable = read!(set.statistics, "attrs_growth");

    /// Reads the values for a regular stat.
    macro_rules! get_stat {
        ($index:literal) => {{
            ShipStat::new()
                .with_base(attrs.get($index)?)
                .with_growth(attrs_growth.get($index)?)
        }};
    }

    let base_list: LuaTable = read!(set.statistics, "base_list");
    let parallel_max: LuaTable = read!(set.statistics, "parallel_max");
    let preload_count: LuaTable = read!(set.statistics, "preload_count");
    let equipment_proficiency: LuaTable = read!(set.statistics, "equipment_proficiency");

    // Intersect the actual and display buff lists so we only include the reasonable
    // ones. This really only matters for Odin currently, whose torpedo
    // adjustment is a separate hidden skill. Usually, hidden buffs end up in
    // `hide_buff_list`.
    let mut buff_list: Vec<u32> = read!(set.template, "buff_list");
    let buff_list_display: Vec<u32> = read!(set.template, "buff_list_display");
    let hide_buff_list: Vec<u32> = read!(set.template, "hide_buff_list");
    buff_list.retain(|i| buff_list_display.contains(i));

    let retriggers = hide_buff_list
        .iter()
        .filter_map(|id| CONFIG.hide_buff_main_gun_retriggers.get(id))
        .copied()
        .max()
        .unwrap_or_default();

    /// Makes an equip slot. The first one specifies the template data.
    /// The second one optionally specifies which index the mount data uses.
    macro_rules! make_equip_slot {
        ($allowed_at:literal, $index:literal) => {{
            EquipSlot {
                allowed: make_equip_slot!(@allowed $allowed_at),
                mount: Some(EquipWeaponMount {
                    efficiency: read!(equipment_proficiency, $index),
                    mounts: read!(base_list, $index),
                    parallel: read!(parallel_max, $index),
                    preload: read!(preload_count, $index),
                    retriggers: if $index == 1 {
                        retriggers
                    } else {
                        0
                    },
                }),
            }
        }};
        ($allowed_at:literal) => {{
            EquipSlot {
                allowed: make_equip_slot!(@allowed $allowed_at),
                mount: None,
            }
        }};
        (@allowed $allowed_at:literal) => {{
            let allow: Vec<u32> = read!(set.template, $allowed_at);
            allow.into_iter().map(convert_al::to_equip_type).collect_fixed_array()
        }};
    }

    let name: String = read!(set.statistics, "name");

    // CMBK: validate only 1 value. currently this simply assumes length 0 or 1 and
    // just take the first item if non-empty
    let specific_type: Vec<String> = read!(set.template, "specific_type");

    let mut ship = ShipData {
        group_id: read!(set.template, "group_type"),
        name: name.trunc_into(),
        rarity: convert_al::to_rarity(read!(set.statistics, "rarity")),
        faction: convert_al::to_faction(read!(set.statistics, "nationality")),
        hull_type: convert_al::to_hull_type(read!(set.statistics, "type")),
        stars: read!(set.template, "star_max"),
        enhance_kind: EnhanceKind::Normal, // overridden below
        stats: ShipStatBlock {
            hp: get_stat!(1),
            armor: convert_al::to_armor_type(read!(set.statistics, "armor_type")),
            rld: get_stat!(6),
            fp: get_stat!(2),
            trp: get_stat!(3),
            eva: get_stat!(9),
            aa: get_stat!(4),
            avi: get_stat!(5),
            acc: get_stat!(8),
            asw: get_stat!(12),
            spd: attrs.get(10)?,
            lck: attrs.get(11)?,
            cost: read!(set.template, "oil_at_end"),
            oxy: read!(set.statistics, "oxy_max"),
            amo: read!(set.statistics, "ammo"),
        },
        default_skin_id: read!(set.statistics, "skin_id"),
        equip_slots: vec![
            make_equip_slot!("equip_1", 1),
            make_equip_slot!("equip_2", 2),
            make_equip_slot!("equip_3", 3),
            make_equip_slot!("equip_4"),
            make_equip_slot!("equip_5"),
        ]
        .trunc_into(),
        shadow_equip: parse::skill::load_wequips(lua, read!(set.statistics, "fix_equip_list"))?
            .into_iter()
            .enumerate()
            .map(|(index, equip)| {
                Ok::<_, LuaError>(ShadowEquip {
                    name: equip.name,
                    efficiency: {
                        let e: Option<f64> = equipment_proficiency.get(4 + index)?;
                        e.unwrap_or(1f64)
                    },
                    weapons: equip.weapons,
                })
            })
            .try_collect_fixed_array()?,
        depth_charges: parse::skill::load_equips(lua, read!(set.statistics, "depth_charge_list"))?
            .trunc_into(),
        skills: parse::skill::load_skills(lua, buff_list)?.trunc_into(),
        ultimate_bonus: specific_type
            .first()
            .map(|s| convert_al::to_ultimate_bonus(s)),
        retrofits: FixedArray::new(), // Added by caller.
        skins: FixedArray::new(),     // Added by caller.
        fleet_tech: None,             // Added by caller.
    };

    if ship.hull_type.team_type() == TeamType::Submarine {
        // I can't explain it but submarine fleet ship costs seem to be 1 too high
        ship.stats.cost -= 1;
    }

    for buff_id in &hide_buff_list {
        if let Some(bonuses) = CONFIG.hide_buff_fixed_stats.get(buff_id) {
            for (stat, amount) in bonuses {
                crate::enhance::add_to_stats_fixed(&mut ship.stats, stat, *amount)?;
            }
        }
    }

    // Patch with the strengthen data.
    match &set.strengthen {
        Strengthen::Normal(data) => {
            // ship_data_strengthen
            ship.enhance_kind = EnhanceKind::Normal;

            fn b(n: f64) -> ShipStat {
                ShipStat::new().with_base(n)
            }

            // Up the base value. This makes stat calc below level 100 inaccurate
            // but I don't really care about that.
            let extra: LuaTable = read!(data, "durability");
            ship.stats.fp += b(extra.get(1)?);
            ship.stats.trp += b(extra.get(2)?);
            ship.stats.aa += b(extra.get(3)?);
            ship.stats.avi += b(extra.get(4)?);
            ship.stats.rld += b(extra.get(5)?);
        },
        Strengthen::Blueprint(ex) => {
            // ship_data_blueprint
            ship.enhance_kind = EnhanceKind::Research;

            let mut effects: Vec<u32> = read!(ex.data, "strengthen_effect");
            effects.append(&mut read!(ex.data, "fate_strengthen"));

            for id in effects {
                enhance::blueprint::add_blueprint_effect(
                    lua,
                    &mut ship,
                    &read!(ex.effect_lookup, id),
                )?;
            }
        },
        Strengthen::Meta(ex) => {
            // ship_strengthen_meta
            ship.enhance_kind = EnhanceKind::Meta;

            for repair_part in [
                "repair_cannon",
                "repair_torpedo",
                "repair_air",
                "repair_reload",
            ] {
                let parts: Vec<u32> = read!(ex.data, repair_part);
                for id in parts {
                    enhance::meta::add_repair(&mut ship, &read!(ex.repair_lookup, id))?;
                }
            }

            let repair_effects: Vec<LuaTable> = read!(ex.data, "repair_effect");
            for table in repair_effects {
                let id: u32 = table.get(2)?;
                enhance::meta::add_repair_effect(&mut ship, &read!(ex.repair_effect_lookup, id))?;
            }

            // META ships have a definition for "buff_list_task" but this seems to go unused
            // and at least Fusou META doesn't even have the right data here. Just use the
            // display list. The skill list will have been mostly empty, so we
            // don't repeat a lot of work here.
            ship.skills = parse::skill::load_skills(lua, buff_list_display)?.trunc_into();
        },
    }

    Ok(ship)
}
