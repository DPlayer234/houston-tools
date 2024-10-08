//! Convenience module for dealing with times and timestamps.

use chrono::prelude::*;

use crate::private::cell::SyncUnsafeCell;

/// Discord's epoch starts at "2015-01-01T00:00:00+00:00"
const DISCORD_EPOCH: u64 = 1_420_070_400_000;

/// Stores a timestamp on when the application was started.
//
// I don't think it's actually possible to cause safety issues on _expected hardware_
// with `DateTime` - it has invariants, but they exist per field and those are small
// enough to have atomic writes/reads - as long as you don't keep references around.
// Either way, it's still UB to Rust, so we treat it with the appropriate care.
static STARTUP_TIME: SyncUnsafeCell<DateTime<Utc>> = SyncUnsafeCell::new(DateTime::UNIX_EPOCH);

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
    unsafe { *STARTUP_TIME.get() = Utc::now(); }
}

/// Gets the marked startup time of the application.
///
/// If the program setup never called [`mark_startup_time`], this will be the unix epoch.
#[must_use]
pub fn get_startup_time() -> DateTime<Utc> {
    // SAFETY: only concurrent reads
    unsafe { *STARTUP_TIME.get() }
}

/// Gets the creation time from a snowflake
#[must_use]
pub fn get_creation_time(snowflake: u64) -> Option<DateTime<Utc>> {
    // This shouldn't be able to fail due to the bit shift, but I'm not validating that.
    #[allow(clippy::cast_possible_wrap)]
    DateTime::from_timestamp_millis(((snowflake >> 22) + DISCORD_EPOCH) as i64)
}

/// Allows mentioning a timestamp in Discord messages.
pub trait TimeMentionable {
    /// Formats a mention for a timestamp.
    fn mention(&self, format: &'static str) -> TimeMention;

    /// Formats a mention with the short time (t) format.
    fn short_time(&self) -> TimeMention { self.mention("t") }
    /// Formats a mention with the long time (T) format.
    fn long_time(&self) -> TimeMention { self.mention("T") }
    /// Formats a mention with the short date (d) format.
    fn short_date(&self) -> TimeMention { self.mention("d") }
    /// Formats a mention with the long date (D) format.
    fn long_date(&self) -> TimeMention { self.mention("D") }
    /// Formats a mention with the short date time (f) format.
    fn short_date_time(&self) -> TimeMention { self.mention("f") }
    /// Formats a mention with the long date time (F) format.
    fn long_date_time(&self) -> TimeMention { self.mention("F") }
    /// Formats a mention with the relative (R) format.
    fn relative(&self) -> TimeMention { self.mention("R") }
}

impl<Tz: TimeZone> TimeMentionable for DateTime<Tz> {
    fn mention(&self, format: &'static str) -> TimeMention {
        TimeMention {
            timestamp: self.timestamp(),
            format,
        }
    }
}

#[derive(Debug, Clone)]
#[must_use]
pub struct TimeMention {
    timestamp: i64,
    format: &'static str,
}

impl std::fmt::Display for TimeMention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<t:{}:{}>", self.timestamp, self.format)
    }
}

/// Tries to parse a date time from some default formats, in the context of a specific time zone.
#[must_use]
pub fn parse_date_time<Tz: TimeZone>(s: &str, tz: Tz) -> Option<DateTime<FixedOffset>> {
    for f in DATE_TIME_FORMATS {
        if let Ok(date_time) = DateTime::parse_from_str(s, f.full) {
            return Some(date_time);
        }

        if let Ok(date_time) = NaiveDateTime::parse_from_str(s, f.naive) {
            return date_time.and_local_timezone(tz)
                .earliest()
                .map(|d| d.fixed_offset());
        }
    }

    None
}

struct DateTimeFormat {
    full: &'static str,
    naive: &'static str
}

macro_rules! make_date_format {
    ($x:expr) => {
        DateTimeFormat {
            full: concat!($x, " %#z"),
            naive: $x
        }
    }
}

const DATE_TIME_FORMATS: &[DateTimeFormat] = &[
    make_date_format!("%Y-%m-%d %H:%M"),
    make_date_format!("%B %d, %Y %H:%M")
];
