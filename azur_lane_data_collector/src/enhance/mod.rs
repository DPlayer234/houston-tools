//! Helper methods to apply enhance/strengthen data.

use azur_lane::ship::*;
use mlua::prelude::*;

pub mod blueprint;
pub mod meta;
pub mod retrofit;

/// Adds to the base amount of the named stat.
pub fn add_to_stats_base(stats: &mut ShipStatBlock, stat: &str, amount: f64) -> LuaResult<()> {
    add_to_stats_intl(stats, stat, amount, ShipStat::new().with_base(amount))
}

/// Adds to the fixed amount of the named stat.
pub fn add_to_stats_fixed(stats: &mut ShipStatBlock, stat: &str, amount: f64) -> LuaResult<()> {
    add_to_stats_intl(stats, stat, amount, ShipStat::new().with_fixed(amount))
}

fn add_to_stats_intl(
    stats: &mut ShipStatBlock,
    stat: &str,
    amount: f64,
    amount_as_stat: ShipStat,
) -> LuaResult<()> {
    match stat {
        "durability" => stats.hp += amount_as_stat,
        "cannon" => stats.fp += amount_as_stat,
        "torpedo" => stats.trp += amount_as_stat,
        "antiaircraft" => stats.aa += amount_as_stat,
        "air" => stats.avi += amount_as_stat,
        "reload" => stats.rld += amount_as_stat,
        "hit" => stats.acc += amount_as_stat,
        "dodge" => stats.eva += amount_as_stat,
        "speed" => stats.spd += amount,
        "luck" => stats.lck += amount,
        "antisub" => stats.asw += amount_as_stat,
        _ => {
            return Err(LuaError::external(anyhow::Error::msg(format!(
                "invalid stat type `{stat}`"
            ))));
        },
    };

    Ok(())
}
