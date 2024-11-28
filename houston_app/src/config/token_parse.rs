//! Serves to actually invoke the [`Token`] validation on deserialization.
//!
//! Not intended to be a lossless round-trip.

use std::fmt;

use serde::de::{Deserializer, Error, Visitor};
use serenity::secrets::Token;

pub fn deserialize<'de, D>(deserializer: D) -> Result<Token, D::Error>
where
    D: Deserializer<'de>,
{
    struct TokenVisitor;
    impl Visitor<'_> for TokenVisitor {
        type Value = Token;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("expected discord token")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            v.parse().map_err(|e| E::custom(e))
        }
    }

    deserializer.deserialize_str(TokenVisitor)
}
