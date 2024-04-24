use std::borrow::Borrow;
use std::sync::Arc;
use mlua::prelude::*;
use azur_lane::ship::*;

use crate::Retrofit;

pub fn apply_retrofit(ship: &mut ShipData, retrofit: &Retrofit) -> LuaResult<()> {
    let list: Vec<Vec<LuaTable>> = retrofit.data.get("transform_list")?;

    for entry in list.iter().flatten() {
        let transform: u32 = entry.get(2)?;
        let transform: LuaTable = retrofit.list_lookup.get(transform)?;
        let effects: Vec<LuaTable> = transform.get("effect")?;
        for effect in effects {
            effect.for_each(|k: String, v: f32| {
                if !super::add_to_stats(&mut ship.stats, &k, v) {
                    match k.borrow() {
                        "skill_id" => { /* todo */ },
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

    Ok(())
}

fn add_equip_efficiency(ship: &mut ShipData, index: usize, amount: f32) -> LuaResult<()> {
    let mut slots = ship.equip_slots.to_vec();
    if let Some(slot) = slots.get_mut(index) {
        slot.efficiency += amount;
        ship.equip_slots = Arc::from(slots);
    }

    Ok(())
}
