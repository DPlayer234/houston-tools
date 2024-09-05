use std::fmt::{Display, Formatter, Result as FmtResult};

use azur_lane::equip::*;
use azur_lane::ship::StatKind;

/// Implements [`Display`] to nicely format a equipment stats.
#[must_use]
pub struct Stats<'a>(StatsInner<'a>);

enum StatsInner<'a> {
    Equip(&'a [EquipStatBonus]),
    Augment(&'a [AugmentStatBonus]),
}

impl<'a> Stats<'a> {
    pub fn equip(equip: &'a Equip) -> Self {
        Self(StatsInner::Equip(&equip.stat_bonuses))
    }

    pub fn augment(augment: &'a Augment) -> Self {
        Self(StatsInner::Augment(&augment.stat_bonuses))
    }
}

impl Display for Stats<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self.0 {
            StatsInner::Equip(a) => write_stats(a, |i| (i.stat_kind, i.amount), f),
            StatsInner::Augment(a) => write_stats(a, |i| (i.stat_kind, i.amount + i.random), f),
        }
    }
}

fn write_stats<I, F>(iter: &[I], map: F, f: &mut Formatter<'_>) -> FmtResult
where
    F: Fn(&I) -> (StatKind, f64),
{
    for (index, chunk) in iter.chunks(3).enumerate() {
        if index != 0 {
            f.write_str("\n")?;
        }

        for (index, stat) in chunk.iter().enumerate() {
            let (kind, amount) = map(stat);
            if index != 0 { f.write_str(" \u{2E31} ")?; }

            let name = kind.name();
            write!(f, "**`{}:`**`{: >len$}`", name, amount, len = 7 - name.len())?;
        }
    }

    Ok(())
}
