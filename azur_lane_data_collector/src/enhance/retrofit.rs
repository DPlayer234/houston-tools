use std::borrow::Borrow;
use std::sync::Arc;
use mlua::prelude::*;
use azur_lane::ship::*;

use crate::Retrofit;
use crate::skill_loader;

pub fn apply_retrofit(lua: &Lua, ship: &mut ShipData, retrofit: &Retrofit) -> LuaResult<()> {
    let list: Vec<Vec<LuaTable>> = retrofit.data.get("transform_list")?;
    let mut new_skills = Vec::new();

    ship.rarity = ship.rarity.next();

    for entry in list.iter().flatten() {
        let transform: u32 = entry.get(2)?;
        let transform: LuaTable = retrofit.list_lookup.get(transform)?;
        let effects: Vec<LuaTable> = transform.get("effect")?;
        if let Some(effect) = effects.last() {
            effect.for_each(|k: String, v: f32| {
                if !super::add_to_stats(&mut ship.stats, &k, v) {
                    match k.borrow() {
                        "skill_id" => new_skills.push(skill_loader::load_skill(lua, v as u32)?),
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
        ship.skills = Arc::from_iter(ship.skills.iter().chain(new_skills.iter()).cloned());
    }

    Ok(())
}

fn add_equip_efficiency(ship: &mut ShipData, index: usize, amount: f32) -> LuaResult<()> {
    let mut slots = ship.equip_slots.to_vec();
    if let Some(slot) = slots.get_mut(index).and_then(|s| s.mount.as_mut()) {
        slot.efficiency += amount;
        ship.equip_slots = Arc::from(slots);
    }

    Ok(())
}
