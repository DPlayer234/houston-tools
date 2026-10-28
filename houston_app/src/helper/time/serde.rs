use std::fmt;

use jiff::SignedDuration;
use serde::Deserializer;
use serde::de::Error;
use serde_with::DeserializeAs;

use super::parse_dhms_duration;

/// Allows deserializing [`Duration`] from a string in the format `d?.hh:mm:ss`.
///
/// This follows the same logic as the [`parse_dhms_duration`] function.
pub enum DhmsDuration {}

impl<'de> DeserializeAs<'de, SignedDuration> for DhmsDuration {
    fn deserialize_as<D>(deserializer: D) -> Result<SignedDuration, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DhmsVisitor;

        impl serde::de::Visitor<'_> for DhmsVisitor {
            type Value = SignedDuration;

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
