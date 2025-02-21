use std::fmt;
use std::num::NonZero;

use arrayvec::ArrayVec;
use chrono::{Datelike as _, Month, NaiveDate};

use crate::modules::model_prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DayOfYear(NonZero<u16>);

impl DayOfYear {
    // this must be a leap-year
    const REF_YEAR: i32 = 2000;
    const FEB_29: Self = Self(NonZero::new(60).unwrap());
    const MAR_1: Self = Self(NonZero::new(61).unwrap());

    pub fn from_md(month: Month, day: u8) -> Option<Self> {
        let date = NaiveDate::from_ymd_opt(Self::REF_YEAR, month.number_from_month(), day.into())?;
        Self::from_ordinal(date.ordinal())
    }

    pub fn search_days(date: NaiveDate) -> ArrayVec<Self, 2> {
        let mut res = ArrayVec::new();
        if let Some(date_ref) = Self::from_date(date) {
            if !date.leap_year() && date_ref == Self::MAR_1 {
                res.push(Self::FEB_29);
            }

            res.push(date_ref);
        }

        res
    }

    fn into_date(self) -> Option<NaiveDate> {
        NaiveDate::from_yo_opt(Self::REF_YEAR, self.0.get().into())
    }

    fn into_month_day(self) -> Option<(Month, u32)> {
        let date = self.into_date()?;
        let month = date.month();
        let month = u8::try_from(month).ok()?;
        let month = Month::try_from(month).ok()?;
        Some((month, date.day()))
    }

    fn from_date(date: NaiveDate) -> Option<Self> {
        let date = date.with_year(Self::REF_YEAR)?;
        Self::from_ordinal(date.ordinal())
    }

    fn from_ordinal(ordinal: u32) -> Option<Self> {
        let ordinal = ordinal.try_into().ok()?;
        let num = NonZero::new(ordinal)?;
        Some(Self(num))
    }
}

impl fmt::Display for DayOfYear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.into_month_day() {
            Some((month, day)) => {
                let suffix = match (day % 10, (10..=20).contains(&day)) {
                    (1, false) => "st",
                    (2, false) => "nd",
                    (3, false) => "rd",
                    _ => "th",
                };

                write!(f, "{} {day}{suffix}", month.name())
            },
            None => f.write_str("<invalid>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consts_correct() {
        assert!(
            NaiveDate::from_yo_opt(DayOfYear::REF_YEAR, 1).is_some_and(|d| d.leap_year()),
            "DayOfYear::REF_YEAR must be a leap year",
        );
        assert_eq!(
            Some(1),
            DayOfYear::from_md(Month::January, 1).map(|n| n.0.get()),
            "jan 1 must be ordinal 1"
        );
        assert_eq!(
            Some(DayOfYear::FEB_29),
            DayOfYear::from_md(Month::February, 29),
            "feb 29 must have the right value"
        );
        assert_eq!(
            Some(DayOfYear::MAR_1),
            DayOfYear::from_md(Month::March, 1),
            "mar 1 must have the right value, one after feb 29"
        );
    }

    #[test]
    fn exact_day_only() {
        assert_eq!(
            DayOfYear::search_days(NaiveDate::from_ymd_opt(2024, 12, 8).unwrap()).as_slice(),
            &[DayOfYear::from_md(Month::December, 8).unwrap()],
        );
        assert_eq!(
            DayOfYear::search_days(NaiveDate::from_ymd_opt(2024, 3, 1).unwrap()).as_slice(),
            &[DayOfYear::from_md(Month::March, 1).unwrap()],
        );
        assert_eq!(
            DayOfYear::search_days(NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()).as_slice(),
            &[DayOfYear::from_md(Month::February, 29).unwrap()],
        );
        assert_eq!(
            DayOfYear::search_days(NaiveDate::from_ymd_opt(2022, 4, 1).unwrap()).as_slice(),
            &[DayOfYear::from_md(Month::April, 1).unwrap()],
        );
    }

    #[test]
    fn non_leap_day_adjustment() {
        assert_eq!(
            DayOfYear::search_days(NaiveDate::from_ymd_opt(2022, 3, 1).unwrap()).as_slice(),
            &[DayOfYear::FEB_29, DayOfYear::MAR_1],
        );
    }
}
