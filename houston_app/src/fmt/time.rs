use std::fmt::{self, Write as _};

use time::Duration;

#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct HumanDuration(Duration);

impl HumanDuration {
    pub fn new(duration: Duration) -> Self {
        Self(duration)
    }
}

impl fmt::Display for HumanDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let left = self.0.whole_seconds();
        if left < 0 {
            f.write_char('-')?;
        }

        fn divrem(a: u64, b: u64) -> (u64, u64) {
            (a / b, a % b)
        }

        let left = left.unsigned_abs();
        let (left, seconds) = divrem(left, 60);
        let (hours, minutes) = divrem(left, 60);

        match (hours, minutes, seconds) {
            (0, 0, _) => write!(f, "{seconds} s"),
            (0, _, 0) => write!(f, "{minutes} m"),
            (_, 0, 0) => write!(f, "{hours} h"),
            (_, _, 0) => write!(f, "{hours}:{minutes:02} h"),
            (_, _, _) => write!(f, "{hours}:{minutes:02}:{seconds:02} h"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds() {
        let t = Duration::seconds(55);
        assert_eq!(HumanDuration(t).to_string(), "55 s");
    }

    #[test]
    fn seconds_neg() {
        let t = Duration::seconds(-55);
        assert_eq!(HumanDuration(t).to_string(), "-55 s");
    }

    #[test]
    fn minutes() {
        let t = Duration::minutes(55);
        assert_eq!(HumanDuration(t).to_string(), "55 m");
    }

    #[test]
    fn minutes_neg() {
        let t = Duration::minutes(-55);
        assert_eq!(HumanDuration(t).to_string(), "-55 m");
    }

    #[test]
    fn hours() {
        let t = Duration::hours(55);
        assert_eq!(HumanDuration(t).to_string(), "55 h");
    }

    #[test]
    fn hours_neg() {
        let t = Duration::hours(-55);
        assert_eq!(HumanDuration(t).to_string(), "-55 h");
    }

    #[test]
    fn hours_minutes() {
        let t = Duration::minutes(261);
        assert_eq!(HumanDuration(t).to_string(), "4:21 h");
    }

    #[test]
    fn hours_minutes_neg() {
        let t = Duration::minutes(-261);
        assert_eq!(HumanDuration(t).to_string(), "-4:21 h");
    }

    #[test]
    fn hours_minutes_seconds() {
        let t = Duration::seconds(3700);
        assert_eq!(HumanDuration(t).to_string(), "1:01:40 h");
    }

    #[test]
    fn hours_minutes_seconds_neg() {
        let t = Duration::seconds(-3700);
        assert_eq!(HumanDuration(t).to_string(), "-1:01:40 h");
    }
}
