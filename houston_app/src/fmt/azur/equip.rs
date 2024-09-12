use std::fmt::{Display, Formatter, Result as FmtResult};

use azur_lane::equip::*;
use azur_lane::ship::StatKind;

/// Implements [`Display`] to nicely format a equipment stats.
#[must_use]
pub struct EquipStats<'a>(&'a Equip);

/// Implements [`Display`] to nicely format a augment stats.
#[must_use]
pub struct AugmentStats<'a>(&'a Augment);

impl<'a> EquipStats<'a> {
    pub fn new(equip: &'a Equip) -> Self {
        Self(equip)
    }
}

impl<'a> AugmentStats<'a> {
    pub fn new(augment: &'a Augment) -> Self {
        Self(augment)
    }
}

impl Display for EquipStats<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write_stats(&self.0.stat_bonuses, |i| (i.stat_kind, i.amount), f)
    }
}

impl Display for AugmentStats<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write_stats(&self.0.stat_bonuses, |i| (i.stat_kind, i.amount + i.random), f)
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
