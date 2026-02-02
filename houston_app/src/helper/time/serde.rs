use std::fmt;

use serde::Deserializer;
use serde::de::Error;
use serde_with::DeserializeAs;
use time::Duration;

use super::parse_dhms_duration;

/// Allows deserializing [`Duration`] from a string in the format `d?.hh:mm:ss`.
///
/// This follows the same logic as the [`parse_dhms_duration`] function.
pub enum DhmsDuration {}

impl<'de> DeserializeAs<'de, Duration> for DhmsDuration {
    fn deserialize_as<D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DhmsVisitor;

        impl serde::de::Visitor<'_> for DhmsVisitor {
            type Value = Duration;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("duration string in hh:mm:ss format")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                parse_dhms_duration(v)
                    .ok_or_else(|| E::custom("expected duration in hh:mm:ss format"))
            }
        }

        deserializer.deserialize_str(DhmsVisitor)
    }
}
