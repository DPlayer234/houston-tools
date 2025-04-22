use std::fmt::{Display, Formatter, Result};

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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write_stats(&self.0.stat_bonuses, |i| (i.stat_kind, i.amount), f)
    }
}

impl Display for AugmentStats<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write_stats(
            &self.0.stat_bonuses,
            |i| (i.stat_kind, i.amount + i.random),
            f,
        )
    }
}

fn write_stats<I, F>(iter: &[I], map: F, f: &mut Formatter<'_>) -> Result
where
    F: Fn(&I) -> (StatKind, f64),
{
    fn write_stat((kind, amount): (StatKind, f64), f: &mut Formatter<'_>) -> Result {
        let name = kind.name();
        let len = 7 - name.len();
        write!(f, "**`{name}:`**`{amount: >len$}`")
    }

    // CMBK: gear never adds more than 3 stats
    // in fact, the game data doesn't allow more than that
    // if this assumption ever changes, re-add chunking lines
    let mut iter = iter.iter();
    if let Some(item) = iter.next() {
        write_stat(map(item), f)?;
        for item in iter {
            f.write_str(" \u{2E31} ")?;
            write_stat(map(item), f)?;
        }
    }

    Ok(())
}
