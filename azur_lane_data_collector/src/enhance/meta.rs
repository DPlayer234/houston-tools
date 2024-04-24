use mlua::prelude::*;
use azur_lane::ship::*;

use crate::context;

pub fn add_repair(ship: &mut ShipData, table: &LuaTable) -> LuaResult<()> {
    let effect: LuaTable = table.get("effect_attr")?;
    
    let attr: String = context!(effect.get(1); "repair's effect_attr name for meta ship id {}", ship.group_id)?;
    let value: f32 = effect.get(2)?;

    super::add_to_stats(&mut ship.stats, &attr, value);

    Ok(())
}

pub fn add_repair_effect(ship: &mut ShipData, table: &LuaTable) -> LuaResult<()> {
    let effect_attr: Vec<LuaTable> = table.get("effect_attr")?;
    for effect in effect_attr {
        let attr: String = context!(effect.get(1); "repair_effect's effect_attr name for meta ship id {}", ship.group_id)?;
        let value: f32 = effect.get(2)?;

        super::add_to_stats(&mut ship.stats, &attr, value);
    }
    
    Ok(())
}
