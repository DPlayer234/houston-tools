use std::fmt;

use chrono::TimeDelta;

#[derive(Debug, Clone, Copy)]
pub struct HumanDuration(TimeDelta);

fn divrem(a: i64, b: i64) -> (i64, i64) {
    (a.div_euclid(b), a.rem_euclid(b))
}

impl HumanDuration {
    pub fn new(duration: TimeDelta) -> Self {
        Self(duration)
    }
}

impl fmt::Display for HumanDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let left = self.0.num_seconds();
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
        let t = TimeDelta::seconds(55);
        assert_eq!(HumanDuration(t).to_string(), "55 s");
    }

    #[test]
    fn minutes() {
        let t = TimeDelta::minutes(55);
        assert_eq!(HumanDuration(t).to_string(), "55 m");
    }

    #[test]
    fn hours() {
        let t = TimeDelta::hours(55);
        assert_eq!(HumanDuration(t).to_string(), "55 h");
    }

    #[test]
    fn hours_minutes() {
        let t = TimeDelta::minutes(261);
        assert_eq!(HumanDuration(t).to_string(), "4:21 h");
    }

    #[test]
    fn hours_minutes_seconds() {
        let t = TimeDelta::seconds(3700);
        assert_eq!(HumanDuration(t).to_string(), "1:01:40 h");
    }
}
