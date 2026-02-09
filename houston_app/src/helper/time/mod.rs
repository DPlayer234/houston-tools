//! Convenience module for dealing with times and timestamps.

use std::sync::OnceLock;

use time::format_description::StaticFormatDescription;
use time::macros::format_description;
use time::{Duration, UtcDateTime};

mod serde;

pub use serde::DhmsDuration;

/// Stores a timestamp on when the application was started.
static STARTUP_TIME: OnceLock<UtcDateTime> = OnceLock::new();

/// Marks the current time as the startup time of the application.
///
/// This should be called once at the start of your `main` entry point. If this
/// function has been called already, it does nothing.
pub fn mark_startup_time() {
    _ = STARTUP_TIME.set(UtcDateTime::now());
}

/// Gets the marked startup time of the application.
///
/// If the program setup never called [`mark_startup_time`], this will return
/// the unix epoch.
#[must_use]
pub fn get_startup_time() -> UtcDateTime {
    STARTUP_TIME
        .get()
        .copied()
        .unwrap_or(UtcDateTime::UNIX_EPOCH)
}

/// Tries to parse a date time from some default formats.
pub fn parse_date_time(s: &str) -> Option<UtcDateTime> {
    UtcDateTime::parse(s, FORMAT).ok()
}

/// This format has 3 distinct sections:
///
/// - Date & Year: "Y-M-D", "M/D/Y", "D.M.Y", or "Month D, Y"
/// - Time: "hh:mm\[:ss\] AM/PM" or "HH:mm\[:ss\]"
/// - UTC-Offset (optional): "+H:mm", "+HHMM", or "+HH"
const FORMAT: StaticFormatDescription = format_description!(
    version = 2,
    "[first \
     [[year]-[month]-[day]]\
     [[month]/[day]/[year]]\
     [[day].[month].[year]]\
     [[first [[month repr:long case_sensitive:false]][[month repr:short case_sensitive:false]]] [day],[optional [ ]][year]]\
     ] \
     [first \
     [[hour repr:12 padding:none]:[minute][optional [:[second]]][optional [ ]][period case_sensitive:false]]
     [[hour repr:24 padding:none]:[minute][optional [:[second]]]]\
     ]\
     [optional [ ]]\
     [first \
     [[offset_hour sign:mandatory padding:none]:[offset_minute]]\
     [[offset_hour sign:mandatory padding:zero][offset_minute]]\
     [[offset_hour sign:mandatory padding:zero]]\
     []\
     ]"
);

/// Parses a [`Duration`] from a string in the format `d?.hh:mm:ss`. If parsing
/// fails, returns [`None`].
///
/// The minute and second components must be between 0 and 59 but may be single
/// digits. There are numeric limits on the hour and day components. If
/// specified, days are considered to be exactly 24 hours. The string may be
/// prefixed with `-` for negative durations.
pub fn parse_dhms_duration(v: &str) -> Option<Duration> {
    use time::convert::{Day, Hour, Minute, Second};

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

    let total = d
        .checked_mul(Hour::per_t(Day))?
        .checked_add(h)?
        .checked_mul(Minute::per_t(Hour))?
        .checked_add(m)?
        .checked_mul(Second::per_t(Minute))?
        .checked_add(s)?;

    let total = if neg { total.checked_neg()? } else { total };
    Some(Duration::seconds(total))
}

#[cfg(test)]
mod tests {
    use time::Duration;
    use time::macros::{datetime, utc_datetime};

    use super::{parse_date_time, parse_dhms_duration};

    #[test]
    fn parse_date_time1() {
        let input = "2024-02-16 15:31";
        let parsed = parse_date_time(input).expect("parse should succeed");

        assert_eq!(parsed, utc_datetime!(2024-02-16 15:31:00));
    }

    #[test]
    fn parse_date_time1s() {
        let input = "2024-02-16 15:31:42";
        let parsed = parse_date_time(input).expect("parse should succeed");

        assert_eq!(parsed, utc_datetime!(2024-02-16 15:31:42));
    }

    #[test]
    fn parse_date_time2() {
        let input = "02/16/2024 03:31pm";
        let parsed = parse_date_time(input).expect("parse should succeed");

        assert_eq!(parsed, utc_datetime!(2024-02-16 15:31:00));
    }

    #[test]
    fn parse_date_time2s() {
        let input = "02/16/2024 3:31:42pm";
        let parsed = parse_date_time(input).expect("parse should succeed");

        assert_eq!(parsed, utc_datetime!(2024-02-16 15:31:42));
    }

    #[test]
    fn parse_date_time3() {
        let input = "16.02.2024 15:31";
        let parsed = parse_date_time(input).expect("parse should succeed");

        assert_eq!(parsed, utc_datetime!(2024-02-16 15:31:00));
    }

    #[test]
    fn parse_date_time4() {
        let input = "February 16, 2024 15:31";
        let parsed = parse_date_time(input).expect("parse should succeed");

        assert_eq!(parsed, utc_datetime!(2024-02-16 15:31:00));
    }

    #[test]
    fn parse_date_time_with_offset1() {
        let input = "2024-02-16 15:31 +0230";
        let parsed = parse_date_time(input).expect("parse should succeed");

        assert_eq!(parsed, datetime!(2024-02-16 15:31:00+02:30).to_utc());
    }

    #[test]
    fn parse_date_time_with_offset2() {
        let input = "2024-02-16 15:31 +2:30";
        let parsed = parse_date_time(input).expect("parse should succeed");

        assert_eq!(parsed, datetime!(2024-02-16 15:31:00+02:30).to_utc());
    }

    #[test]
    fn parse_date_time_with_offset3() {
        let input = "2024-02-16 15:31 +02";
        let parsed = parse_date_time(input).expect("parse should succeed");

        assert_eq!(parsed, datetime!(2024-02-16 15:31:00+02:00).to_utc());
    }

    #[test]
    fn fail_parse_date_partial_time() {
        let input = "2024-02-16 11:";
        assert!(parse_date_time(input).is_none(), "parse should fail");
    }

    #[test]
    fn fail_parse_date_invalid_time() {
        let input = "2024-02-16 11:61";
        assert!(parse_date_time(input).is_none(), "parse should fail");
    }

    #[test]
    fn fail_parse_partial_date() {
        let input = "13/13/ 14:15";
        assert!(parse_date_time(input).is_none(), "parse should fail");
    }

    #[test]
    fn fail_parse_invalid_date() {
        let input = "13/13/13 14:15";
        assert!(parse_date_time(input).is_none(), "parse should fail");
    }

    #[test]
    fn fail_parse_date_time_invalid_time_zone() {
        let input = "2024-02-12 11:51 CET";
        assert!(parse_date_time(input).is_none(), "parse should fail");
    }

    #[test]
    fn parse_serde_time_delta_hms() {
        let input = "12:34:56";
        let parsed = parse_dhms_duration(input).expect("parse should succeed");
        assert_eq!(parsed, Duration::seconds(((12 * 60 + 34) * 60) + 56));
    }

    #[test]
    fn parse_serde_time_delta_dhms() {
        let input = "8.12:34:56";
        let parsed = parse_dhms_duration(input).expect("parse should succeed");
        assert_eq!(
            parsed,
            Duration::seconds((((8 * 24 + 12) * 60 + 34) * 60) + 56)
        );
    }
}
