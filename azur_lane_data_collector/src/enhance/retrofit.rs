use std::borrow::Borrow;

use mlua::prelude::*;

use azur_lane::ship::*;

use crate::parse;
use crate::Retrofit;

/// Applies the full retrofit template to the ship data.
pub fn apply_retrofit(lua: &Lua, ship: &mut ShipData, retrofit: &Retrofit) -> LuaResult<()> {
    let list: Vec<Vec<LuaTable>> = retrofit.data.get("transform_list")?;
    let mut new_skills = Vec::new();

    ship.rarity = ship.rarity.next();

    for entry in list.iter().flatten() {
        let transform: u32 = entry.get(2)?;
        let transform: LuaTable = retrofit.list_lookup.get(transform)?;

        // If not zero, override the default skin ID value.
        let skin_id: u32 = transform.get("skin_id")?;
        if skin_id != 0 {
            ship.default_skin_id = skin_id;
        }

        // Effects are structured as a list of maps,
        // where the nested map keys are the effect type and its value the amount.
        let effects: Vec<LuaTable> = transform.get("effect")?;
        for effect in effects {
            effect.for_each(|k: String, v: f64| {
                // Stats added by retrofits are NOT affected by affinity.
                if !super::add_to_stats_fixed(&mut ship.stats, &k, v) {
                    match k.borrow() {
                        "skill_id" => new_skills.push(parse::skill::load_skill(lua, v as u32)?),
                        "equipment_proficiency_1" => add_equip_efficiency(ship, 0, v)?,
                        "equipment_proficiency_2" => add_equip_efficiency(ship, 1, v)?,
                        "equipment_proficiency_3" => add_equip_efficiency(ship, 2, v)?,
                        _ => ()
                    }
                }

                Ok(())
            })?;
        }
    }

    if !new_skills.is_empty() {
        ship.skills.extend(new_skills.into_iter());
    }

    Ok(())
}

fn add_equip_efficiency(ship: &mut ShipData, index: usize, amount: f64) -> LuaResult<()> {
    if let Some(slot) = ship.equip_slots.get_mut(index).and_then(|s| s.mount.as_mut()) {
        slot.efficiency += amount;
    }

    Ok(())
}
