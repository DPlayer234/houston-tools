use std::sync::Arc;
use mlua::prelude::*;
use azur_lane::ship::*;

use crate::context;
use crate::skill_loader;

pub fn add_blueprint_effect(lua: &Lua, ship: &mut ShipData, table: &LuaTable) -> LuaResult<()> {
    const M: f32 = 100f32;

    let effect: LuaTable = table.get("effect")?;
    ship.stats.fp += { let v: f32 = effect.get(1)?; v / M };
    ship.stats.trp += { let v: f32 = effect.get(2)?; v / M };
    ship.stats.aa += { let v: f32 = effect.get(3)?; v / M };
    ship.stats.avi += { let v: f32 = effect.get(4)?; v / M };
    ship.stats.rld += { let v: f32 = effect.get(5)?; v / M };

    if let LuaValue::Table(effect_attr) = table.get("effect_attr")? {
        add_effect_attr(ship, effect_attr)?;
    }

    if let LuaValue::Table(change_skill) = table.get("change_skill")? {
        replace_skill(lua, ship, change_skill)?;
    }
    
    if let LuaValue::Table(effect_base) = table.get("effect_base")? {
        replace_equip_slot_part(lua, ship, effect_base, |s| &mut s.mounts)?;
    }

    if let LuaValue::Table(effect_preload) = table.get("effect_preload")? {
        replace_equip_slot_part(lua, ship, effect_preload, |s| &mut s.preload)?;
    }

    if let LuaValue::Table(equip_efficiency) = table.get("effect_equipment_proficiency")? {
        add_equip_efficiency(ship, equip_efficiency)?;
    }

    Ok(())
}

fn add_effect_attr(ship: &mut ShipData, effect_attr: LuaTable) -> LuaResult<()> {
    effect_attr.for_each(|_: u32, v: LuaTable| {
        let attr: String = context!(v.get(1); "effect_attr name for blueprint ship id {}", ship.group_id)?;
        let value: f32 = v.get(2)?;

        super::add_to_stats(&mut ship.stats, &attr, value);

        Ok(())
    })
}

fn replace_skill(lua: &Lua, ship: &mut ShipData, effect: LuaTable) -> LuaResult<()> {
    let from_id: u32 = effect.get(1)?;
    let to_id: u32 = effect.get(2)?;

    let mut skills = ship.skills.to_vec();
    if let Some(slot) = skills.iter_mut().find(|s| s.buff_id == from_id) {
        *slot = skill_loader::load_skill(lua, to_id)?;
        ship.skills = Arc::from(skills);
    }

    Ok(())
}

fn replace_equip_slot_part<'a, T: FromLua<'a> + Clone>(lua: &'a Lua, ship: &mut ShipData, effect: LuaTable<'a>, select: impl Fn(&mut EquipWeaponMount) -> &mut T) -> LuaResult<()> {
    let mut slots = ship.equip_slots.to_vec();
    let effect_base: Vec<T> = Vec::from_lua(LuaValue::Table(effect), lua)?;

    for (index, mounts) in effect_base.iter().enumerate() {
        if let Some(slot) = slots.get_mut(index).and_then(|s| s.mount.as_mut()) {
            *select(slot) = mounts.clone();
        }
    }

    ship.equip_slots = Arc::from(slots);
    Ok(())
}

fn add_equip_efficiency(ship: &mut ShipData, effect: LuaTable) -> LuaResult<()> {
    let index: usize = effect.get(1)?;
    let amount: f32 = effect.get(2)?;

    let mut slots = ship.equip_slots.to_vec();
    if let Some(slot) = slots.get_mut(index - 1).and_then(|s| s.mount.as_mut()) {
        slot.efficiency += amount;
        ship.equip_slots = Arc::from(slots);
    }

    Ok(())
}
