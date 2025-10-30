//! Convenience module for dealing with times and timestamps.

use std::fmt;

use chrono::TimeDelta;
use serde::Deserializer;
use serde::de::Error;
use serde_with::DeserializeAs;

/// Marker type to use with [`serde_with`] in place of a [`TimeDelta`] to
/// deserialize it from a string.
pub enum TimeDeltaStr {}

impl<'de> DeserializeAs<'de, TimeDelta> for TimeDeltaStr {
    fn deserialize_as<D>(deserializer: D) -> Result<TimeDelta, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = TimeDelta;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("duration string in hh:mm:ss format")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                TimeDeltaStr::parse(v)
                    .ok_or_else(|| E::custom("expected duration in hh:mm:ss format"))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl TimeDeltaStr {
    fn parse(v: &str) -> Option<TimeDelta> {
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
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;

    use super::TimeDeltaStr;

    #[test]
    fn parse_serde_time_delta_hms() {
        let input = "12:34:56";
        let parsed = TimeDeltaStr::parse(input).expect("parse should succeed");
        assert_eq!(parsed, TimeDelta::seconds(((12 * 60 + 34) * 60) + 56));
    }

    #[test]
    fn parse_serde_time_delta_dhms() {
        let input = "8.12:34:56";
        let parsed = TimeDeltaStr::parse(input).expect("parse should succeed");
        assert_eq!(
            parsed,
            TimeDelta::seconds((((8 * 24 + 12) * 60 + 34) * 60) + 56)
        );
    }
}
