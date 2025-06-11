use std::fmt;
use std::num::NonZero;

use arrayvec::ArrayVec;
use chrono::{Datelike as _, Month, NaiveDate};

/// Represents a day of a year, without a specific year number.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DayOfYear(
    /// The leap year day ordinal.
    // ideally this would be `u16 @ 1..=366`
    // code may panic if the value is outside this range
    NonZero<u16>,
);

impl DayOfYear {
    /// The reference year used to calculate and relate ordinals.
    ///
    /// This must be a leap year.
    const REF_YEAR: i32 = 2000;

    /// The value for February 29th.
    const FEB_29: Self = Self::new(60);
    /// The value for March 1st.
    const MAR_1: Self = Self::new(61);

    /// Creates a new [`DayOfYear`] with the specified leap-year ordinal.
    ///
    /// # Panics
    ///
    /// Panics if the ordinal is not in the allowed range.
    const fn new(ordinal: u16) -> Self {
        Self::new_checked(ordinal).expect("date ordinal must in range 1..=366")
    }

    /// Creates a new [`DayOfYear`] with the specified leap-year ordinal.
    ///
    /// # Errors
    ///
    /// Returns [`None`] if the ordinal is not in the allowed range.
    const fn new_checked(ordinal: u16) -> Option<Self> {
        match NonZero::new(ordinal) {
            Some(i) if i.get() <= 366 => Some(Self(i)),
            _ => None,
        }
    }

    /// Gets the leap-year ordinal.
    const fn ordinal(self) -> u16 {
        self.0.get()
    }

    /// Creates a [`DayOfYear`] from a month and day.
    ///
    /// # Errors
    ///
    /// Returns [`None`] when the `day` is zero or not valid for `month` in a
    /// leap year.
    pub fn from_md(month: Month, day: u8) -> Option<Self> {
        let date = NaiveDate::from_ymd_opt(Self::REF_YEAR, month.number_from_month(), day.into())?;
        let ordinal = expect_ordinal(date.ordinal());
        Some(Self::new(ordinal))
    }

    /// Creates a [`DayOfYear`] from a date.
    pub fn from_date(date: NaiveDate) -> Self {
        let mut ordinal = expect_ordinal(date.ordinal());
        if !date.leap_year() && ordinal >= Self::FEB_29.ordinal() {
            debug_assert!(ordinal <= 365, "non-leap year ordinal cannot be 366");
            ordinal += 1;
        }

        Self::new(ordinal)
    }

    /// Given a date, returns all days considered to "match" it in the current
    /// year.
    ///
    /// In practice, all this does is return the associated [`DayOfYear`], and,
    /// if it is March 1st in a non-leap year, also returns February 29th.
    pub fn search_days(date: NaiveDate) -> ArrayVec<Self, 2> {
        let mut res = ArrayVec::new();
        let doy = Self::from_date(date);

        if !date.leap_year() && doy == Self::MAR_1 {
            res.push(Self::FEB_29);
        }

        res.push(doy);
        res
    }

    /// Converts this day into a date in the [`Self::REF_YEAR`].
    fn into_date(self) -> NaiveDate {
        NaiveDate::from_yo_opt(Self::REF_YEAR, self.ordinal().into())
            .expect("DayOfYear value must be valid in REF_YEAR")
    }

    /// Converts this day into the Month and Day it represents.
    fn into_month_day(self) -> (Month, u32) {
        let date = self.into_date();
        let month = date.month();
        let month = u8::try_from(month).expect("month number must fit into `u8`");
        let month = Month::try_from(month).expect("month number must be convertible to `Month`");
        (month, date.day())
    }
}

/// Converts [`u32`] to a [`u16`], assuming that the value is in range for a
/// date ordinal.
#[expect(clippy::cast_possible_truncation)]
fn expect_ordinal(ordinal: u32) -> u16 {
    debug_assert!(ordinal <= 366, "expected date ordinal in range 1..=366");
    ordinal as u16
}

impl serde::Serialize for DayOfYear {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

// manually implemented to respect the type's valid range
impl<'de> serde::Deserialize<'de> for DayOfYear {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        let ordinal = u16::deserialize(deserializer)?;
        Self::new_checked(ordinal).ok_or_else(|| D::Error::custom("expected u16 in range 1..=366"))
    }
}

impl fmt::Display for DayOfYear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (month, day) = self.into_month_day();

        let suffix = match day {
            1 | 21 | 31 => "st",
            2 | 22 => "nd",
            3 | 23 => "rd",
            _ => "th",
        };

        let month = month.name();
        write!(f, "{month} {day}{suffix}")
    }
}

impl fmt::Debug for DayOfYear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

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
            DayOfYear::search_days(date(2024, 12, 8)).as_slice(),
            &[DayOfYear::from_md(Month::December, 8).expect("checked day should be valid")],
        );
        assert_eq!(
            DayOfYear::search_days(date(2024, 3, 1)).as_slice(),
            &[DayOfYear::from_md(Month::March, 1).expect("checked day should be valid")],
        );
        assert_eq!(
            DayOfYear::search_days(date(2024, 2, 29)).as_slice(),
            &[DayOfYear::from_md(Month::February, 29).expect("checked day should be valid")],
        );
        assert_eq!(
            DayOfYear::search_days(date(2022, 4, 1)).as_slice(),
            &[DayOfYear::from_md(Month::April, 1).expect("checked day should be valid")],
        );
        assert_eq!(
            DayOfYear::search_days(date(2022, 1, 1)).as_slice(),
            &[DayOfYear::from_md(Month::January, 1).expect("checked day should be valid")],
        );
        assert_eq!(
            DayOfYear::search_days(date(2024, 1, 1)).as_slice(),
            &[DayOfYear::from_md(Month::January, 1).expect("checked day should be valid")],
        );
        assert_eq!(
            DayOfYear::search_days(date(2022, 12, 31)).as_slice(),
            &[DayOfYear::from_md(Month::December, 31).expect("checked day should be valid")],
        );
        assert_eq!(
            DayOfYear::search_days(date(2024, 12, 31)).as_slice(),
            &[DayOfYear::from_md(Month::December, 31).expect("checked day should be valid")],
        );
    }

    #[test]
    fn non_leap_day_adjustment() {
        assert_eq!(
            DayOfYear::search_days(date(2022, 3, 1)).as_slice(),
            &[DayOfYear::FEB_29, DayOfYear::MAR_1],
        );
    }
}
