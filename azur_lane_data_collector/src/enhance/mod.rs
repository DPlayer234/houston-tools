use azur_lane::ship::*;

pub mod blueprint;
pub mod meta;
pub mod retrofit;

pub fn add_to_stats_base(stats: &mut ShipStats, stat: &str, amount: f32) -> bool {
    add_to_stats_intl(stats, stat, amount, ShipStatValue::new(amount, 0f32, 0f32))
}

pub fn add_to_stats_fixed(stats: &mut ShipStats, stat: &str, amount: f32) -> bool {
    add_to_stats_intl(stats, stat, amount, ShipStatValue::new(0f32, 0f32, amount))
}

fn add_to_stats_intl(stats: &mut ShipStats, stat: &str, amount: f32, amount_as_stat: ShipStatValue) -> bool {
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
        _ => { return false; }
    };

    true
}
