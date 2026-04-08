use azur_lane::ship::*;
use mlua::prelude::*;

use crate::intl_util::FixedArrayExt as _;
use crate::{RetrofitSet, parse};

/// Applies the full retrofit template to the ship data.
pub fn apply_retrofit(lua: &Lua, ship: &mut Retrofit, retrofit: &RetrofitSet<'_>) -> LuaResult<()> {
    let list: LuaTable = retrofit.data.get("transform_list")?;

    ship.base.rarity = ship.base.rarity.next();

    // the outer list are the nodes in the retrofit tree, the inner lists are the
    // different steps. every step applies individually rather than replacing the
    // previous ones.
    for outer in list.sequence_values() {
        let outer: LuaTable = outer?;
        for entry in outer.sequence_values() {
            let entry: LuaTable = entry?;
            let transform: u32 = entry.get(2)?;
            let transform: LuaTable = retrofit.list_lookup.get(transform)?;

            // If not zero, override the default skin ID value.
            let skin_id: u32 = transform.get("skin_id")?;
            if skin_id != 0 {
                ship.base.default_skin_id = skin_id;
            }

            // Effects are structured as a list of maps,
            // where the nested map keys are the effect type and its value the amount.
            let effects: LuaTable = transform.get("effect")?;
            for effect in effects.sequence_values() {
                let effect: LuaTable = effect?;
                effect.for_each(|k, v| add_effect(lua, ship, k, v))?;
            }
        }
    }

    Ok(())
}

fn add_effect(lua: &Lua, ship: &mut Retrofit, k: LuaBorrowedStr<'_>, v: f64) -> LuaResult<()> {
    // Stats added by retrofits are NOT affected by affinity.
    if super::add_to_stats_fixed(&mut ship.base.stats, &k, v).is_err() {
        match &*k {
            "skill_id" =>
            {
                #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                ship.base
                    .skills
                    .push(parse::skill::load_skill(lua, v as u32)?)
            },
            "equipment_proficiency_1" => add_equip_efficiency(&mut ship.base, 0, v)?,
            "equipment_proficiency_2" => add_equip_efficiency(&mut ship.base, 1, v)?,
            "equipment_proficiency_3" => add_equip_efficiency(&mut ship.base, 2, v)?,
            _ => (),
        }
    }

    Ok(())
}

fn add_equip_efficiency(ship: &mut BaseShip, index: usize, amount: f64) -> LuaResult<()> {
    if let Some(slot) = ship.equip_slots.get_mut(index)
        && let Some(mount) = &mut slot.mount
    {
        mount.efficiency += amount;
    }

    Ok(())
}
