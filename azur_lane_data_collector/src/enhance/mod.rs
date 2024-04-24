use azur_lane::ship::*;

pub mod blueprint;
pub mod meta;
pub mod retrofit;

pub fn add_to_stats(stats: &mut ShipStats, stat: &str, amount: f32) -> bool {
    match stat {
        "durability" => stats.hp += amount,
        "cannon" => stats.fp += amount,
        "torpedo" => stats.trp += amount,
        "antiaircraft" => stats.aa += amount,
        "air" => stats.avi += amount,
        "reload" => stats.rld += amount,
        "hit" => stats.acc += amount,
        "dodge" => stats.eva += amount,
        "speed" => stats.spd += amount,
        "luck" => stats.lck += amount,
        "antisub" => stats.asw += amount,
        _ => { return false; }
    };

    true
}
