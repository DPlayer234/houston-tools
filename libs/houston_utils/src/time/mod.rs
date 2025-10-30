//! Convenience module for dealing with times and timestamps.

use std::cell::UnsafeCell;

use chrono::format::Item;
use chrono::prelude::*;

pub mod fmt;
mod serde;

pub use serde::TimeDeltaStr;

// basically SyncUnsafeCell<DateTime<Utc>>
struct DateTimeCell {
    value: UnsafeCell<DateTime<Utc>>,
}

// SAFETY: consumers must uphold aliasing requirements for the inner value.
unsafe impl Sync for DateTimeCell {}

/// Stores a timestamp on when the application was started.
//
// I don't think it's actually possible to cause safety issues on _expected
// hardware_ with `DateTime` - it has invariants, but they exist per field and
// those are small enough to have atomic writes/reads - as long as you don't
// keep references around. Either way, it's still UB to Rust, so we treat it
// with the appropriate care.
static STARTUP_TIME: DateTimeCell = DateTimeCell {
    value: UnsafeCell::new(DateTime::UNIX_EPOCH),
};

/// Marks the current time as the startup time of the application.
///
/// This should be called once at the start of your `main` entry point.
///
/// # Safety
///
/// This function is unsafe as the underlying memory is static.
/// This must not be called concurrently with itself or [`get_startup_time`].
pub unsafe fn mark_startup_time() {
    // SAFETY: Caller guarantees exclusive access
    unsafe {
        *STARTUP_TIME.value.get() = Utc::now();
    }
}

/// Gets the marked startup time of the application.
///
/// If the program setup never called [`mark_startup_time`], this will be the
/// unix epoch.
#[must_use]
pub fn get_startup_time() -> DateTime<Utc> {
    // SAFETY: only concurrent reads
    unsafe { *STARTUP_TIME.value.get() }
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

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::parse_date_time;

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
}
