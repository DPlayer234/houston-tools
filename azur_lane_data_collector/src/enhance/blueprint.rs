use mlua::prelude::*;

use azur_lane::ship::*;

use crate::context;
use crate::parse;

/// Modifies the ship data, adding a blueprint effect.
///
/// This refers to a single enhance/fate simulation level.
pub fn add_blueprint_effect(lua: &Lua, ship: &mut ShipData, table: &LuaTable) -> LuaResult<()> {
    fn b(n: f64) -> ShipStat { ShipStat::new().set_base(n * 0.01) }

    let effect: LuaTable = table.get("effect")?;
    ship.stats.fp += b(effect.get(1)?);
    ship.stats.trp += b(effect.get(2)?);
    ship.stats.aa += b(effect.get(3)?);
    ship.stats.avi += b(effect.get(4)?);
    ship.stats.rld += b(effect.get(5)?);

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

/// "effect_attr" adds a flat amount of base stats.
fn add_effect_attr(ship: &mut ShipData, effect_attr: LuaTable) -> LuaResult<()> {
    effect_attr.for_each(|_: u32, v: LuaTable| {
        let attr: String = context!(v.get(1); "effect_attr name for blueprint ship id {}", ship.group_id)?;
        let value: f64 = v.get(2)?;

        super::add_to_stats_base(&mut ship.stats, &attr, value);

        Ok(())
    })
}

/// "change_skill" replaces the skill with a given ID with another one.
fn replace_skill(lua: &Lua, ship: &mut ShipData, effect: LuaTable) -> LuaResult<()> {
    let from_id: u32 = effect.get(1)?;
    let to_id: u32 = effect.get(2)?;

    if let Some(slot) = ship.skills.iter_mut().find(|s| s.buff_id == from_id) {
        *slot = parse::skill::load_skill(lua, to_id)?;
    }

    Ok(())
}

/// "effect_base" and "effect_preload" *replace* components of the ship's equipment slots.
fn replace_equip_slot_part<'a>(lua: &'a Lua, ship: &mut ShipData, effect: LuaTable<'a>, select: impl Fn(&mut EquipWeaponMount) -> &mut u8) -> LuaResult<()> {
    let effect_base: Vec<u8> = Vec::from_lua(LuaValue::Table(effect), lua)?;

    for (index, &new) in effect_base.iter().enumerate() {
        if let Some(slot) = ship.equip_slots.get_mut(index).and_then(|s| s.mount.as_mut()) {
            let part = select(slot);
            if *part < new { *part = new; }
        }
    }

    Ok(())
}

/// "effect_equipment_proficiency" adds efficiency to some gear slot.
fn add_equip_efficiency(ship: &mut ShipData, effect: LuaTable) -> LuaResult<()> {
    let index: usize = effect.get(1)?;
    let amount: f64 = effect.get(2)?;

    if let Some(slot) = ship.equip_slots.get_mut(index - 1).and_then(|s| s.mount.as_mut()) {
        slot.efficiency += amount;
    }

    Ok(())
}
