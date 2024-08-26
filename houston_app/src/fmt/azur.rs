use std::fmt::{Display, Formatter, Result as FmtResult};

use azur_lane::equip::*;

/// Implements [`Display`] to nicely format a weapon.
///
/// Alternate formatting (`{:#}`) omits the weapon kind.
#[must_use]
pub struct DisplayWeapon<'a>(&'a Weapon);

impl<'a> DisplayWeapon<'a> {
    /// Creates a new value.
    pub fn new(weapon: &'a Weapon) -> Self {
        Self(weapon)
    }

    /// Formats the weapon without the kind.
    pub fn to_string_no_kind(&self) -> String {
        format!("{:#}", self)
    }
}

impl Display for DisplayWeapon<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let weapon = self.0;

        if !f.alternate() {
            writeln!(f, "**Kind:** {}", weapon.kind.name())?;
        }

        format_fire_rate(weapon, f)?;

        match &weapon.data {
            WeaponData::Bullets(barrage) => format_barrage(barrage, f, ""),
            WeaponData::Aircraft(aircraft) => format_aircraft(aircraft, f),
            WeaponData::AntiAir(barrage) => format_anti_air(barrage, f, ""),
        }
    }
}

fn format_fire_rate(weapon: &Weapon, f: &mut Formatter<'_>) -> FmtResult {
    let salvo_time = match &weapon.data {
        WeaponData::Bullets(b) => b.salvo_time,
        _ => 0.0
    };

    let reload_time = weapon.reload_time * if weapon.kind == WeaponKind::StrikeAircraft { 2.2 } else { 1.0 };
    let fixed_delay = weapon.fixed_delay + salvo_time;
    writeln!(
        f,
        "**FR:** {:.2} +{:.2}s (~{:.1}/min)",
        reload_time, fixed_delay, 60.0 / (reload_time + fixed_delay)
    )
}

fn format_barrage(barrage: &Barrage, f: &mut Formatter<'_>, indent: &str) -> FmtResult {
    if barrage.bullets.is_empty() {
        return Ok(());
    }

    let bullet = &barrage.bullets[0];
    let amount: u32 = barrage.bullets.iter().map(|b| b.amount).sum();
    let ArmorModifiers(l, m, h) = bullet.modifiers;

    // amount x damage
    // range | angle | vel
    write!(
        f,
        "{indent}**Dmg:** {} x {:.1} @ {:.0}% {}\n\
        {indent}**Range:** {:.0} \u{2E31} **Angle:** {:.0}° \u{2E31} **Vel.:** {:.0}\n",
        amount, barrage.damage * barrage.coefficient, barrage.scaling * 100f64, barrage.scaling_stat.name(),
        barrage.range, barrage.firing_angle, bullet.velocity
    )?;

    if let Some(spread) = &bullet.spread {
        writeln!(
            f,
            "{indent}**AoE:** {:.0} \u{2E31} **Spread:** {:.0} x {:.0}",
            spread.hit_range, spread.spread_x, spread.spread_y
        )?;
    }

    // ammo type & mods
    write!(
        f,
        "{indent}**{: >4}:** {:.0}/{:.0}/{:.0}",
        bullet.ammo.name(), l * 100f64, m * 100f64, h * 100f64
    )
}

fn format_anti_air(barrage: &Barrage, f: &mut Formatter<'_>, indent: &str) -> FmtResult {
    // damage
    // ammo type & mods
    // range | angle
    write!(
        f,
        "{indent}**Dmg:** {:.1} @ {:.0}% {}\n\
        {indent}**Range:** {:.1} \u{2E31} **Angle:** {:.1}\n",
        barrage.damage * barrage.coefficient, barrage.scaling * 100f64, barrage.scaling_stat.name(),
        barrage.range, barrage.firing_angle,
    )
}

fn format_aircraft(aircraft: &Aircraft, f: &mut Formatter<'_>) -> FmtResult {
    const PAD: &str = "> ";

    writeln!(
        f,
        "**Speed:** {:.0} \u{2E31} **HP:** {:.0} \u{2E31} {}",
        aircraft.speed, aircraft.health.calc(120, 1.0), aircraft.dodge_limit
    )?;

    for weapon in &aircraft.weapons {
        writeln!(f, "__**{}:**__", weapon.name.as_deref().unwrap_or(weapon.kind.name()))?;

        match &weapon.data {
            WeaponData::Bullets(barrage) => {
                format_barrage(barrage, f, PAD)?;
            }
            WeaponData::AntiAir(barrage) => {
                f.write_str(PAD)?;
                format_fire_rate(weapon, f)?;
                format_anti_air(barrage, f, PAD)?;
            }
            WeaponData::Aircraft(..) => {
                f.write_str("<matryoshka aircraft>\n")?;
            }
        }
    }

    Ok(())
}
