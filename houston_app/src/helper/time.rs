//! Convenience module for dealing with times and timestamps.

use std::sync::OnceLock;

use chrono::format::Item;
use chrono::prelude::*;

/// Stores a timestamp on when the application was started.
static STARTUP_TIME: OnceLock<DateTime<Utc>> = OnceLock::new();

/// Marks the current time as the startup time of the application.
///
/// This should be called once at the start of your `main` entry point. If this
/// function has been called already, it does nothing.
pub fn mark_startup_time() {
    _ = STARTUP_TIME.set(Utc::now());
}

/// Gets the marked startup time of the application.
///
/// If the program setup never called [`mark_startup_time`], this will return
/// the unix epoch.
#[must_use]
pub fn get_startup_time() -> DateTime<Utc> {
    STARTUP_TIME.get().copied().unwrap_or(DateTime::UNIX_EPOCH)
}

/// Tries to parse a date time from some default formats, in the context of a
/// specific time zone.
pub fn parse_date_time<Tz: TimeZone>(s: &str, tz: Tz) -> Option<DateTime<FixedOffset>> {
    use chrono::format::{Parsed, parse_and_remainder};

    let formats = &FORMATS;

    fn parse_section<'s>(
        parsed: &mut Parsed,
        parts: &[&[Item<'_>]],
        s: &'s str,
    ) -> Option<&'s str> {
        let backup = parsed.clone();

        for &f in parts {
            if let Ok(s) = parse_and_remainder(parsed, s, f.iter()) {
                return Some(s);
            }

            *parsed = backup.clone();
        }

        None
    }

    let mut parsed = Parsed::new();

    // parse the date & time and consume the input
    let s = parse_section(&mut parsed, formats.date, s)?;
    let s = parse_section(&mut parsed, formats.time, s)?;

    // if the input already entirely consumed, it has no time zone
    // assume the time zone passed to this function is to be used
    if s.is_empty() {
        // we do it like this rather than `to_datetime_with_timezone` to still be able
        // to return a value when it's ambigious due to DST. this use case isn't _that_
        // error sensitive
        return parsed
            .to_naive_datetime_with_offset(0)
            .ok()?
            .and_local_timezone(tz)
            .earliest()
            .map(|d| d.fixed_offset());
    }

    // nothing expected after time zone so it must be fully consumed
    let s = parse_section(&mut parsed, formats.tz, s)?;
    if s.is_empty() {
        return parsed.to_datetime().ok();
    }

    None
}

struct Formats {
    pub date: &'static [&'static [Item<'static>]],
    pub time: &'static [&'static [Item<'static>]],
    pub tz: &'static [&'static [Item<'static>]],
}

const FORMATS: Formats = {
    use chrono::format::Item::{Fixed, Literal};
    use chrono::format::{Fixed as F, Numeric as N, Pad};

    const fn space() -> Item<'static> {
        Item::Space(" ")
    }

    const fn num_pz(n: N) -> Item<'static> {
        Item::Numeric(n, Pad::Zero)
    }

    const fn simple_date(a: N, b: N, c: N, sep: &str) -> [Item<'_>; 7] {
        [
            space(),
            num_pz(a),
            Literal(sep),
            num_pz(b),
            Literal(sep),
            num_pz(c),
            space(),
        ]
    }

    Formats {
        date: &[
            // %Y-%m-%d
            &simple_date(N::Year, N::Month, N::Day, "-"),
            // %m/%d/%Y
            &simple_date(N::Month, N::Day, N::Year, "/"),
            // %d.%m.%Y
            &simple_date(N::Day, N::Month, N::Year, "."),
            // %B %d, %Y
            &[
                space(),
                Fixed(F::LongMonthName),
                space(),
                num_pz(N::Day),
                Literal(","),
                space(),
                num_pz(N::Year),
                space(),
            ],
        ],
        // 12-hour format must come first so the 24-hour format doesn't succeed on parts of it
        // and then causes parsing failures later when the AM/PM is encountered
        time: &[
            // %I:%M %p
            &[
                space(),
                num_pz(N::Hour12),
                Literal(":"),
                num_pz(N::Minute),
                space(),
                Fixed(F::UpperAmPm),
                space(),
            ],
            // %H:%M
            &[
                space(),
                num_pz(N::Hour),
                Literal(":"),
                num_pz(N::Minute),
                space(),
            ],
        ],
        tz: &[
            // emulates "%#z" by trying multiple formats
            &[space(), Fixed(F::TimezoneOffsetZ), space()],
            &[space(), Fixed(F::TimezoneOffsetTripleColon), space()],
        ],
    }
};

pub mod serde_time_delta {
    use std::fmt;

    use chrono::TimeDelta;
    use serde::Deserializer;
    use serde::de::Error;

    struct Visitor;

    pub(super) fn parse_str(v: &str) -> Option<TimeDelta> {
        let v = v.trim();
        let (v, neg) = match v.strip_prefix('-') {
            Some(v) => (v, true),
            None => (v, false),
        };

        if v.contains('-') {
            return None;
        }

        let (h, v) = v.split_once(':')?;
        let (m, s) = v.split_once(':')?;

        let (d, h) = match h.split_once('.') {
            Some((d, h)) => (d.parse().ok()?, h),
            None => (0i64, h),
        };

        let h = h.parse().ok()?;
        let m = m.parse().ok()?;
        let s = s.parse().ok()?;

        if !(0..60).contains(&m) || !(0..60).contains(&s) {
            return None;
        }

        let delta = TimeDelta::try_days(d)?
            .checked_add(&TimeDelta::try_hours(h)?)?
            .checked_add(&TimeDelta::try_minutes(m)?)?
            .checked_add(&TimeDelta::try_seconds(s)?)?;

        Some(if neg { -delta } else { delta })
    }

    impl serde::de::Visitor<'_> for Visitor {
        type Value = TimeDelta;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("duration string in hh:mm:ss format")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            parse_str(v).ok_or_else(|| E::custom("expected duration in hh:mm:ss format"))
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<TimeDelta, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeDelta, Utc};

    use super::{parse_date_time, serde_time_delta};

    #[test]
    fn parse_date_time1() {
        let input = "2024-02-16 15:31";
        let parsed = parse_date_time(input, Utc).expect("parse should succeed");

        assert_eq!(
            parsed,
            DateTime::parse_from_rfc3339("2024-02-16T15:31:00Z").expect("must be valid")
        );
    }

    #[test]
    fn parse_date_time2() {
        let input = "02/16/2024 03:31pm";
        let parsed = parse_date_time(input, Utc).expect("parse should succeed");

        assert_eq!(
            parsed,
            DateTime::parse_from_rfc3339("2024-02-16T15:31:00Z").expect("must be valid")
        );
    }

    #[test]
    fn parse_date_time3() {
        let input = "16.02.2024 15:31";
        let parsed = parse_date_time(input, Utc).expect("parse should succeed");

        assert_eq!(
            parsed,
            DateTime::parse_from_rfc3339("2024-02-16T15:31:00Z").expect("must be valid")
        );
    }

    #[test]
    fn parse_date_time4() {
        let input = "February 16, 2024 15:31";
        let parsed = parse_date_time(input, Utc).expect("parse should succeed");

        assert_eq!(
            parsed,
            DateTime::parse_from_rfc3339("2024-02-16T15:31:00Z").expect("must be valid")
        );
    }

    #[test]
    fn parse_date_time_with_offset() {
        let input = "2024-02-16 15:31 +0230";
        let parsed = parse_date_time(input, Utc).expect("parse should succeed");

        assert_eq!(
            parsed,
            DateTime::parse_from_rfc3339("2024-02-16T15:31:00+02:30").expect("must be valid")
        );
    }

    #[test]
    fn fail_parse_date_partial_time() {
        let input = "2024-02-16 11:";
        assert!(parse_date_time(input, Utc).is_none(), "parse should fail");
    }

    #[test]
    fn fail_parse_date_invalid_time() {
        let input = "2024-02-16 11:61";
        assert!(parse_date_time(input, Utc).is_none(), "parse should fail");
    }

    #[test]
    fn fail_parse_partial_date() {
        let input = "13/13/ 14:15";
        assert!(parse_date_time(input, Utc).is_none(), "parse should fail");
    }

    #[test]
    fn fail_parse_invalid_date() {
        let input = "13/13/13 14:15";
        assert!(parse_date_time(input, Utc).is_none(), "parse should fail");
    }

    #[test]
    fn fail_parse_date_time_invalid_time_zone() {
        let input = "2024-02-12 11:51 CET";
        assert!(parse_date_time(input, Utc).is_none(), "parse should fail");
    }

    #[test]
    fn parse_serde_time_delta_hms() {
        let input = "12:34:56";
        let parsed = serde_time_delta::parse_str(input).expect("parse should succeed");
        assert_eq!(parsed, TimeDelta::seconds(((12 * 60 + 34) * 60) + 56));
    }

    #[test]
    fn parse_serde_time_delta_dhms() {
        let input = "8.12:34:56";
        let parsed = serde_time_delta::parse_str(input).expect("parse should succeed");
        assert_eq!(
            parsed,
            TimeDelta::seconds((((8 * 24 + 12) * 60 + 34) * 60) + 56)
        );
    }
}
