//! Convenience module for dealing with times and timestamps.

use std::sync::RwLock;

use serenity::model::Timestamp;
use serenity::utils::Content;
use chrono::prelude::*;

pub const SHORT_TIME: Option<char> = Some('t');
pub const LONG_TIME: Option<char> = Some('T');
pub const SHORT_DATE: Option<char> = Some('d');
pub const LONG_DATE: Option<char> = Some('D');
pub const SHORT_DATE_TIME: Option<char> = Some('f');
pub const LONG_DATE_TIME: Option<char> = Some('F');
pub const RELATIVE: Option<char> = Some('R');

/// Stores a timestamp on when the bot was started.
static STARTUP_TIME: RwLock<Option<DateTime<Utc>>> = RwLock::new(None);

/// Allows mentioning a timestamp in Discord messages.
pub trait TimestampMention {
	/// Formats a mention for a timestamp.
	fn mention(&self, format: Option<char>) -> Content;
}

/// Marks the current time as the startup time of the application.
pub fn mark_startup_time() {
	let mut l = STARTUP_TIME.write().expect("startup_time mutex poisoned");
	*l = Some(Utc::now());
}

/// Gets the marked startup time of the application.
/// Will be [None] if unset.
pub fn get_startup_time() -> Option<DateTime<Utc>> {
	*STARTUP_TIME.read().expect("startup_time mutex poisoned")
}

impl TimestampMention for Timestamp {
	fn mention(&self, format: Option<char>) -> Content {
		if let Some(format_raw) = format {
			format!("<t:{}:{}>", self.timestamp(), format_raw).into()
		} else {
			format!("<t:{}>", self.timestamp()).into()
		}
	}
}
