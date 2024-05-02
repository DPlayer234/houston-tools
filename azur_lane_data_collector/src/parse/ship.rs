use mlua::prelude::*;
use azur_lane::ship::*;

use crate::context;
use crate::convert_al;
use crate::enhance;
use crate::parse;
use crate::model::*;

/// Constructs ship data from this set.
pub fn load_ship_data(lua: &Lua, set: &ShipSet) -> LuaResult<ShipData> {
    /// Reads a single value; target-typed.
    macro_rules! read {
        ($table:expr, $field:expr) => {
            context!($table.get($field); "{} of ship with id {}", $field, set.id)?
        };
    }

    let attrs: LuaTable = read!(set.statistics, "attrs");
    let attrs_growth: LuaTable = read!(set.statistics, "attrs_growth");

    /// Reads the values for a regular stat.
    macro_rules! get_stat {
        ($index:literal) => {{
            ShipStat::new()
                .set_base(attrs.get($index)?)
                .set_growth(attrs_growth.get($index)?)
        }};
    }

    /// Reads the values for a stat without level growth.
    macro_rules! get_non_grow_stat {
        ($index:literal) => {{
            let base: f64 = attrs.get($index)?;
            base
        }};
    }

    let base_list: LuaTable = read!(set.statistics, "base_list");
    let parallel_max: LuaTable = read!(set.statistics, "parallel_max");
    let preload_count: LuaTable = read!(set.statistics, "preload_count");
    let equipment_proficiency: LuaTable = read!(set.statistics, "equipment_proficiency");

    // Intersect the actual and display buff lists so we only include the reasonable ones.
    // This really only matters for Odin currently, whose torpedo adjustment is a separate hidden skill.
    // Usually, hidden buffs end up in `hide_buff_list`.
    let mut buff_list: Vec<u32> = read!(set.template, "buff_list");
    let buff_list_display: Vec<u32> = read!(set.template, "buff_list_display");
    let hide_buff_list: Vec<u32> = read!(set.template, "hide_buff_list");
    intersect(&mut buff_list, &buff_list_display);

    // Speaking of, skill 1 is BB MGM+1 and skill 2 is BB MGM+2, so let's just hard-code this.
    // The actual skill data re-fires the weapon, so it would work as a multiplier.
    let main_mount_mult: u8 =
        if hide_buff_list.contains(&1) { 2 }
        else if hide_buff_list.contains(&2) { 3 }
        else { 1 };

    /// Makes an equip slot. The first one specifies the template data.
    /// The second one optionally specifies which index the mount data uses.
    macro_rules! make_equip_slot {
        ($allowed_at:literal, $index:literal) => {{
            let allow: Vec<u32> = read!(set.template, $allowed_at);
            let mut mounts: u8 = read!(base_list, $index);
            if $index == 1 { mounts *= main_mount_mult; }
            
            EquipSlot {
                allowed: allow.iter().map(|&n| convert_al::to_equip_type(n)).collect(),
                mount: Some(EquipWeaponMount {
                    efficiency: read!(equipment_proficiency, $index),
                    mounts,
                    parallel: read!(parallel_max, $index),
                    preload: read!(preload_count, $index)
                })
            }
        }};
        ($allowed_at:literal) => {{
            let allow: Vec<u32> = read!(set.template, $allowed_at);
            EquipSlot {
                allowed: allow.iter().map(|&n| convert_al::to_equip_type(n)).collect(),
                mount: None
            }
        }};
    }

    let mut ship = ShipData {
        group_id: read!(set.template, "group_type"),
        name: read!(set.statistics, "name"), 
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
            spd: get_non_grow_stat!(10),
            lck: get_non_grow_stat!(11),
            cost: read!(set.template, "oil_at_end"),
            oxy: read!(set.statistics, "oxy_max"),
            amo: read!(set.statistics, "ammo")
        },
        default_skin_id: read!(set.statistics, "skin_id"),
        equip_slots: vec![
            make_equip_slot!("equip_1", 1),
            make_equip_slot!("equip_2", 2),
            make_equip_slot!("equip_3", 3),
            make_equip_slot!("equip_4"),
            make_equip_slot!("equip_5")
        ],
        shadow_equip: parse::skill::load_equips(lua, read!(set.statistics, "fix_equip_list"))?.into_iter()
            .enumerate()
            .map(|(index, equip)| Ok(ShadowEquip {
                name: equip.name,
                efficiency: { let e: Option<f64> = equipment_proficiency.get(4 + index)?; e.unwrap_or(1f64) },
                weapons: equip.weapons
            }))
            .collect::<LuaResult<Vec<_>>>()?,
        skills: parse::skill::load_skills(lua, buff_list)?,
        retrofits: Vec::new(), // Added by caller.
        skins: Vec::new() // Added by caller.
    };

    if ship.hull_type.team_type() == TeamType::Submarine {
        // I can't explain it but submarine fleet ship costs seem to be 1 too high
        ship.stats.cost -= 1;
    }

    // Patch with the strengthen data.
    match &set.strengthen {
        Strengthen::Normal(data) => {
            // ship_data_strengthen
            ship.enhance_kind = EnhanceKind::Normal;

            fn b(n: f64) -> ShipStat { ShipStat::new().set_base(n) }
            
            // Up the base value. This makes stat calc below level 100 inaccurate
            // but I don't really care about that.
            let extra: LuaTable = read!(data, "durability");
            ship.stats.fp += b(extra.get(1)?);
            ship.stats.trp += b(extra.get(2)?);
            ship.stats.aa += b(extra.get(3)?);
            ship.stats.avi += b(extra.get(4)?);
            ship.stats.rld += b(extra.get(5)?);
        }
        Strengthen::Blueprint(ex) => {
            // ship_data_blueprint
            ship.enhance_kind = EnhanceKind::Research;

            let mut effects: Vec<u32> = read!(ex.data, "strengthen_effect");
            effects.append(&mut read!(ex.data, "fate_strengthen"));

            for id in effects {
                enhance::blueprint::add_blueprint_effect(lua, &mut ship, &read!(ex.effect_lookup, id))?;
            }
        }
        Strengthen::META(ex) => {
            // ship_strengthen_meta
            ship.enhance_kind = EnhanceKind::META;

            for repair_part in ["repair_cannon", "repair_torpedo", "repair_air", "repair_reload"] {
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
            // and at least Fusou META doesn't even have the right data here. Just use the display list.
            // The skill list will have been mostly empty, so we don't repeat a lot of work here.
            ship.skills = parse::skill::load_skills(lua, buff_list_display)?;
        }
    }

    Ok(ship)
}

/// Intersects a vector with a slice.
/// In other words, removes all elements from the vector that aren't in the slice.
fn intersect<T: Eq>(target: &mut Vec<T>, other: &[T]) {
    // This isn't really efficient, but it's easy and works.
    let mut try_again = true;
    while try_again {
        try_again = false;
        for (index, item) in target.iter().enumerate() {
            if !other.contains(item) {
                target.remove(index);
                try_again = true;
                break;
            }
        }
    }
}
