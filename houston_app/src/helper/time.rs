//! Convenience module for dealing with times and timestamps.

use std::cell::UnsafeCell;

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
#[must_use]
pub fn parse_date_time<Tz: TimeZone>(s: &str, tz: Tz) -> Option<DateTime<FixedOffset>> {
    for f in DATE_TIME_FORMATS {
        if let Ok(date_time) = DateTime::parse_from_str(s, f.full) {
            return Some(date_time);
        }

        if let Ok(date_time) = NaiveDateTime::parse_from_str(s, f.naive) {
            return date_time
                .and_local_timezone(tz)
                .earliest()
                .map(|d| d.fixed_offset());
        }
    }

    None
}

struct DateTimeFormat {
    full: &'static str,
    naive: &'static str,
}

macro_rules! make_date_format {
    ($x:expr) => {
        DateTimeFormat {
            full: concat!($x, " %#z"),
            naive: $x,
        }
    };
}

const DATE_TIME_FORMATS: &[DateTimeFormat] = &[
    make_date_format!("%Y-%m-%d %H:%M"),
    make_date_format!("%B %d, %Y %H:%M"),
];
