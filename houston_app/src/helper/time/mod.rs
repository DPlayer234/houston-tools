//! Convenience module for dealing with times and timestamps.

use std::sync::OnceLock;

use jiff::{SignedDuration, Timestamp};

mod serde;

pub use serde::DhmsDuration;

/// Stores a timestamp on when the application was started.
static STARTUP_TIME: OnceLock<Timestamp> = OnceLock::new();

/// Marks the current time as the startup time of the application.
///
/// This should be called once at the start of your `main` entry point. If this
/// function has been called already, it does nothing.
pub fn mark_startup_time() {
    _ = STARTUP_TIME.set(Timestamp::now());
}

/// Gets the marked startup time of the application.
///
/// If the program setup never called [`mark_startup_time`], this will return
/// the unix epoch.
#[must_use]
pub fn get_startup_time() -> Timestamp {
    STARTUP_TIME.get().copied().unwrap_or(Timestamp::UNIX_EPOCH)
}

/// Parses a [`Duration`] from a string in the format `d?.hh:mm:ss`. If parsing
/// fails, returns [`None`].
///
/// The minute and second components must be between 0 and 59 but may be single
/// digits. There are numeric limits on the hour and day components. If
/// specified, days are considered to be exactly 24 hours. The string may be
/// prefixed with `-` for negative durations.
pub fn parse_dhms_duration(v: &str) -> Option<SignedDuration> {
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

    if m.len() != 2 || s.len() != 2 {
        return None;
    }

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
        .checked_mul(24)?
        .checked_add(h)?
        .checked_mul(60)?
        .checked_add(m)?
        .checked_mul(60)?
        .checked_add(s)?;

    let total = if neg { total.checked_neg()? } else { total };
    Some(SignedDuration::from_secs(total))
}

#[cfg(test)]
mod tests {
    use jiff::SignedDuration;

    use super::parse_dhms_duration;

    #[test]
    fn parse_duration_hms() {
        let input = "12:34:56";
        let parsed = parse_dhms_duration(input).expect("parse should succeed");
        assert_eq!(
            parsed,
            SignedDuration::from_secs(((12 * 60 + 34) * 60) + 56)
        );
    }

    #[test]
    fn parse_duration_nhms() {
        let input = "-12:34:56";
        let parsed = parse_dhms_duration(input).expect("parse should succeed");
        assert_eq!(
            parsed,
            -SignedDuration::from_secs(((12 * 60 + 34) * 60) + 56)
        );
    }

    #[test]
    fn parse_duration_dhms() {
        let input = "8.12:34:56";
        let parsed = parse_dhms_duration(input).expect("parse should succeed");
        assert_eq!(
            parsed,
            SignedDuration::from_secs((((8 * 24 + 12) * 60 + 34) * 60) + 56)
        );
    }

    #[test]
    fn parse_duration_ndhms() {
        let input = "-8.12:34:56";
        let parsed = parse_dhms_duration(input).expect("parse should succeed");
        assert_eq!(
            parsed,
            -SignedDuration::from_secs((((8 * 24 + 12) * 60 + 34) * 60) + 56)
        );
    }

    #[test]
    fn parse_duration_fail() {
        assert!(parse_dhms_duration("22:44").is_none(), "missing component");
        assert!(
            parse_dhms_duration("22:00:60").is_none(),
            "second out of range"
        );
        assert!(
            parse_dhms_duration("22:60:00").is_none(),
            "minute out of range"
        );
        assert!(
            parse_dhms_duration("*22:00:00").is_none(),
            "invalid character"
        );
        assert!(
            parse_dhms_duration("22.00:00").is_none(),
            "invalid character"
        );
        assert!(
            parse_dhms_duration("22:00.00").is_none(),
            "invalid character"
        );
        assert!(
            parse_dhms_duration("22:00:00:00").is_none(),
            "too many components"
        );
        assert!(
            parse_dhms_duration("22:00:000").is_none(),
            "seconds too wide"
        );
        assert!(
            parse_dhms_duration("22:000:00").is_none(),
            "minutes too wide"
        );
        assert!(parse_dhms_duration("22:00:0").is_none(), "seconds too thin");
        assert!(parse_dhms_duration("22:0:00").is_none(), "minutes too thin");
        assert!(
            parse_dhms_duration("106751991167300.15:30:07").is_some()
                && parse_dhms_duration("-106751991167300.15:30:07").is_some(),
            "this should be in range"
        );
        assert!(
            parse_dhms_duration("106751991167300.15:30:08").is_none(),
            "overflow"
        );
        assert!(
            parse_dhms_duration("-106751991167300.15:30:08").is_none(),
            "underflow"
        );
    }
}
