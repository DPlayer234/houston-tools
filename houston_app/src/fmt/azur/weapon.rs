use std::fmt::{Display, Formatter, Result};

use azur_lane::equip::*;

/// Implements [`Display`] to nicely format a weapon.
#[must_use]
pub struct Details<'a> {
    weapon: &'a Weapon,
    flags: DetailFlags,
}

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    #[repr(transparent)]
    struct DetailFlags: usize {
        const NO_KIND = 1 << 0;
        const NO_FIRE_RATE = 1 << 1;
    }
}

impl<'a> Details<'a> {
    /// Creates a new value.
    pub const fn new(weapon: &'a Weapon) -> Self {
        Self {
            weapon,
            flags: DetailFlags::empty(),
        }
    }

    pub fn no_kind(mut self) -> Self {
        self.flags.insert(DetailFlags::NO_KIND);
        self
    }

    pub fn no_fire_rate(mut self) -> Self {
        self.flags.insert(DetailFlags::NO_FIRE_RATE);
        self
    }
}

impl Display for Details<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let weapon = self.weapon;

        if !self.flags.contains(DetailFlags::NO_KIND) {
            writeln!(f, "**Kind:** {}", weapon.kind.name())?;
        }

        if !self.flags.contains(DetailFlags::NO_FIRE_RATE) {
            format_fire_rate(weapon, f)?;
        }

        match &weapon.data {
            WeaponData::Bullets(barrage) => format_barrage(barrage, f, ""),
            WeaponData::Aircraft(aircraft) => format_aircraft(aircraft, f),
            WeaponData::AntiAir(barrage) => format_anti_air(barrage, f, ""),
        }
    }
}

fn format_fire_rate(weapon: &Weapon, f: &mut Formatter<'_>) -> Result {
    let salvo_time = match &weapon.data {
        WeaponData::Bullets(b) => b.salvo_time,
        _ => 0.0,
    };

    let weapon_kind_reload_mult = if weapon.kind == WeaponKind::StrikeAircraft {
        2.2
    } else {
        1.0
    };

    let reload_time = weapon.reload_time * weapon_kind_reload_mult;
    let fixed_delay = weapon.fixed_delay + salvo_time;
    writeln!(
        f,
        "**FR:** {:.2} +{:.2}s (~{:.1}/min)",
        reload_time,
        fixed_delay,
        60.0 / (reload_time + fixed_delay),
    )
}

fn format_barrage(barrage: &Barrage, f: &mut Formatter<'_>, indent: &str) -> Result {
    if barrage.bullets.is_empty() {
        return Ok(());
    }

    let bullet = &barrage.bullets[0];
    let amount: u32 = barrage.bullets.iter().map(|b| b.amount).sum();
    let ArmorModifiers(l, m, h) = bullet.modifiers;

    match &bullet.extra {
        // ticks x amount x damage
        BulletExtra::Beam(beam) => writeln!(
            f,
            "{indent}**Dmg:** ~{:.0} x {} x {:.1} @ {:.0}% {}",
            beam.duration / beam.tick_delay,
            amount,
            barrage.damage * barrage.coefficient,
            barrage.scaling * 100f64,
            barrage.scaling_stat.name(),
        )?,
        // amount x damage
        _ => writeln!(
            f,
            "{indent}**Dmg:** {} x {:.1} @ {:.0}% {}",
            amount,
            barrage.damage * barrage.coefficient,
            barrage.scaling * 100f64,
            barrage.scaling_stat.name(),
        )?,
    }

    // range | angle | vel
    writeln!(
        f,
        "{indent}**Range:** {:.0} \u{2E31} **Angle:** {:.0}° \u{2E31} **Vel.:** {:.0}",
        barrage.range, barrage.firing_angle, bullet.velocity
    )?;

    if let BulletExtra::Spread(spread) = &bullet.extra {
        writeln!(
            f,
            "{indent}**AoE:** {:.0} \u{2E31} **Spread:** {:.0} x {:.0}",
            spread.hit_range, spread.spread_x, spread.spread_y
        )?;
    }

    // ammo type & mods
    write!(
        f,
        "{indent}**{}:** {:.0}/{:.0}/{:.0}",
        bullet.ammo.name(),
        l * 100f64,
        m * 100f64,
        h * 100f64,
    )?;

    // hits both surface and subs
    if bullet.flags.dive_filter().is_empty() {
        write!(f, "\n{indent}-# *Hits Submarines*")?;
    }

    Ok(())
}

fn format_anti_air(barrage: &Barrage, f: &mut Formatter<'_>, indent: &str) -> Result {
    // damage
    // ammo type & mods
    // range | angle
    write!(
        f,
        "{indent}**Dmg:** {:.1} @ {:.0}% {}\n\
         {indent}**Range:** {:.1} \u{2E31} **Angle:** {:.1}\n",
        barrage.damage * barrage.coefficient,
        barrage.scaling * 100f64,
        barrage.scaling_stat.name(),
        barrage.range,
        barrage.firing_angle,
    )
}

fn format_aircraft(aircraft: &Aircraft, f: &mut Formatter<'_>) -> Result {
    const PAD: &str = "> ";

    writeln!(
        f,
        "**Speed:** {:.0} \u{2E31} **HP:** {:.0} \u{2E31} {}",
        aircraft.speed,
        aircraft.health.calc(120, 1.0),
        aircraft.dodge_limit,
    )?;

    for weapon in &aircraft.weapons {
        writeln!(
            f,
            "__**{}:**__",
            weapon.name.as_deref().unwrap_or_else(|| weapon.kind.name())
        )?;

        match &weapon.data {
            WeaponData::Bullets(barrage) => {
                format_barrage(barrage, f, PAD)?;
            },
            WeaponData::AntiAir(barrage) => {
                f.write_str(PAD)?;
                format_fire_rate(weapon, f)?;
                format_anti_air(barrage, f, PAD)?;
            },
            WeaponData::Aircraft(..) => {
                f.write_str("<matryoshka aircraft>\n")?;
            },
        }
    }

    Ok(())
}
