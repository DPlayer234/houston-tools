//! Convenience module for dealing with times and timestamps.

use std::cell::UnsafeCell;
use std::sync::LazyLock;

use arrayvec::ArrayVec;
use chrono::format::Item;
use chrono::prelude::*;

// basically SyncUnsafeCell<DateTime<Utc>>
struct DateTimeCell {
    value: UnsafeCell<DateTime<Utc>>,
}

unsafe impl Sync for DateTimeCell {}

impl DateTimeCell {
    const fn unix_epoch() -> Self {
        Self {
            value: UnsafeCell::new(DateTime::UNIX_EPOCH),
        }
    }

    const fn get(&self) -> *mut DateTime<Utc> {
        self.value.get()
    }
}

/// Stores a timestamp on when the application was started.
//
// I don't think it's actually possible to cause safety issues on _expected
// hardware_ with `DateTime` - it has invariants, but they exist per field and
// those are small enough to have atomic writes/reads - as long as you don't
// keep references around. Either way, it's still UB to Rust, so we treat it
// with the appropriate care.
static STARTUP_TIME: DateTimeCell = DateTimeCell::unix_epoch();

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
        *STARTUP_TIME.get() = Utc::now();
    }
}

/// Gets the marked startup time of the application.
///
/// If the program setup never called [`mark_startup_time`], this will be the
/// unix epoch.
#[must_use]
pub fn get_startup_time() -> DateTime<Utc> {
    // SAFETY: only concurrent reads
    unsafe { *STARTUP_TIME.get() }
}

/// Tries to parse a date time from some default formats, in the context of a
/// specific time zone.
pub fn parse_date_time<Tz: TimeZone>(s: &str, tz: Tz) -> Option<DateTime<FixedOffset>> {
    use chrono::format::{parse, parse_and_remainder, Parsed};

    let formats = &*FORMATS;
    for f in &formats.date_time {
        let mut parsed = Parsed::new();

        // parse the date & time, keeping the remainder
        let Ok(s) = parse_and_remainder(&mut parsed, s, f.iter()) else {
            continue;
        };

        // input already entirely consumed, so it has no time zone
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

        for tz in &formats.tz {
            if parse(&mut parsed, s, tz.iter()).is_ok() {
                return parsed.to_datetime().ok();
            }
        }
    }

    None
}

// honestly i wanted to const-construct these instead of invoking parsing logic
// but some of the item variants cannot be publicly constructed
struct Formats {
    pub date_time: [ArrayVec<Item<'static>, 13>; 4],
    pub tz: [ArrayVec<Item<'static>, 3>; 1],
}

static FORMATS: LazyLock<Formats> = LazyLock::new(|| {
    use chrono::format::StrftimeItems;

    // like `StrftimeItems::parse` but collects into `ArrayVec` and panics on error
    fn create<const N: usize>(s: &'static str) -> ArrayVec<Item<'static>, N> {
        StrftimeItems::new(s)
            .inspect(|i| assert_ne!(*i, Item::Error, "date time format is invalid"))
            .collect()
    }

    // if this is changed, make sure to ensure `Formats` is updated to still have
    // enough space for all format items. run the tests to be sure also
    Formats {
        date_time: [
            create(" %Y-%m-%d %H:%M "),
            create(" %m/%d/%Y %I:%M %p "),
            create(" %d.%m.%Y %H:%M "),
            create(" %B %d, %Y %H:%M "),
        ],
        tz: [create(" %#z ")],
    }
});

pub mod serde_time_delta {
    use std::fmt;

    use chrono::TimeDelta;
    use serde::de::Error;
    use serde::Deserializer;

    struct Visitor;

    fn parse_str(v: &str) -> Option<TimeDelta> {
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
    use chrono::{DateTime, Utc};

    #[test]
    fn parse_date_time1() {
        let input = "2024-03-12 15:31";
        let parsed = super::parse_date_time(input, Utc).unwrap();

        assert_eq!(
            parsed,
            DateTime::parse_from_rfc3339("2024-03-12T15:31:00Z").unwrap()
        );
    }

    #[test]
    fn parse_date_time2() {
        let input = "03/12/2024 03:31pm";
        let parsed = super::parse_date_time(input, Utc).unwrap();

        assert_eq!(
            parsed,
            DateTime::parse_from_rfc3339("2024-03-12T15:31:00Z").unwrap()
        );
    }

    #[test]
    fn parse_date_time3() {
        let input = "12.03.2024 15:31";
        let parsed = super::parse_date_time(input, Utc).unwrap();

        assert_eq!(
            parsed,
            DateTime::parse_from_rfc3339("2024-03-12T15:31:00Z").unwrap()
        );
    }

    #[test]
    fn parse_date_time4() {
        let input = "March 12, 2024 15:31";
        let parsed = super::parse_date_time(input, Utc).unwrap();

        assert_eq!(
            parsed,
            DateTime::parse_from_rfc3339("2024-03-12T15:31:00Z").unwrap()
        );
    }

    #[test]
    fn parse_date_time_with_offset() {
        let input = "2024-03-12 15:31 +0230";
        let parsed = super::parse_date_time(input, Utc).unwrap();

        assert_eq!(
            parsed,
            DateTime::parse_from_rfc3339("2024-03-12T15:31:00+02:30").unwrap()
        );
    }
}
