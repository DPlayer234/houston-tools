use std::fmt::{Display, Formatter, Result as FmtResult};

use azur_lane::equip::*;

pub struct WeaponFormat<'a>(&'a Weapon);

impl<'a> WeaponFormat<'a> {
    pub fn new(weapon: &'a Weapon) -> Self {
        Self(weapon)
    }
}

impl Display for WeaponFormat<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let weapon = &self.0;

        write!(f, "__*{}*__\n", weapon.kind.name())?;
        format_fire_rate(weapon, f)?;

        match &weapon.data {
            WeaponData::Bullets(barrage) => format_barrage(barrage, f, ""),
            WeaponData::Aircraft(aircraft) => format_aircraft(aircraft, f),
            WeaponData::AntiAir(barrage) => format_anti_air(barrage, f, ""),
        }
    }
}

fn format_fire_rate(weapon: &Weapon, f: &mut Formatter<'_>) -> FmtResult {
    let fixed_delay = weapon.fixed_delay;
    write!(
        f,
        "**FR:** {:.2} +{:.2}s (~{:.1}/min)\n",
        weapon.reload_time, fixed_delay, 60.0 / (weapon.reload_time + fixed_delay)
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
    // ammo type & mods
    // range | angle | vel
    write!(
        f,
        "{indent}**Dmg:** {} x {:.1} @ {:.0}% {}\n\
        {indent}**{: >4}:** {:.0}/{:.0}/{:.0}\n\
        {indent}**Range:** {:.1} \u{2E31} **Angle:** {:.1} \u{2E31} **Vel.:** {:.1}\n",
        amount, barrage.damage * barrage.coefficient, barrage.scaling * 100f64, barrage.scaling_stat.name(),
        bullet.ammo.short_name(), l * 100f64, m * 100f64, h * 100f64,
        barrage.range, barrage.firing_angle, bullet.velocity
    )
}

fn format_anti_air(barrage: &Barrage, f: &mut Formatter<'_>, indent: &str) -> FmtResult {
    let amount: u32 = barrage.bullets.iter().map(|b| b.amount).sum();

    // amount x damage
    // ammo type & mods
    // range | angle
    write!(
        f,
        "{indent}**Dmg:** {} x {:.1} @ {:.0}% {}\n\
        {indent}**Range:** {:.1} \u{2E31} **Angle:** {:.1}\n",
        amount, barrage.damage * barrage.coefficient, barrage.scaling * 100f64, barrage.scaling_stat.name(),
        barrage.range, barrage.firing_angle,
    )
}

fn format_aircraft(aircraft: &Aircraft, f: &mut Formatter<'_>) -> FmtResult {
    const PAD: &str = "> ";

    for weapon in &aircraft.weapons {
        write!(f, "{PAD}__*{}*__\n", weapon.kind.name())?;

        match &weapon.data {
            WeaponData::Bullets(barrage) => {
                format_barrage(barrage, f, PAD)?;
            }
            WeaponData::AntiAir(barrage) => {
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
