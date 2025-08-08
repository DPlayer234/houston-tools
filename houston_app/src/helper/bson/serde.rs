//! Helper functions for serializing values in BSON.
//!
//! This primarily deals with serializing [serenity]'s IDs as integers.

use std::marker::PhantomData;

use serde::de::{Error, Visitor};
use serde::{Deserializer, Serializer};
use serde_with::{DeserializeAs, SerializeAs};

trait CastI64: From<u64> + Into<i64> + Copy {}
impl<T: From<u64> + Into<i64> + Copy> CastI64 for T {}

/// Serializes Discord IDs' underlying [`u64`] value as [`i64`], and reverses
/// that process for deserialization.
///
/// When deserializing, also accepts stringified [`u64`] values.
pub enum IdI64 {}

impl<T: CastI64> SerializeAs<T> for IdI64 {
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let int = (*source).into();
        serializer.serialize_i64(int)
    }
}

impl<'de, T: CastI64> DeserializeAs<'de, T> for IdI64 {
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(I64Visitor(PhantomData))
    }
}

struct I64Visitor<T>(PhantomData<T>);

impl<T: CastI64> Visitor<'_> for I64Visitor<T> {
    type Value = T;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a string or integer snowflake")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_u64(v.cast_unsigned())
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        // serenity's ids panic if you try to construct them from `u64::MAX`
        // since they are backed by `NonMaxU64` inner values.
        if v != u64::MAX {
            Ok(T::from(v))
        } else {
            Err(E::custom("discord id cannot be u64::MAX"))
        }
    }

    // need to support from-str deserialization also due to old code invoking the
    // `MessageId`'s own Serialize impl that defaults to serializing as strings.
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        v.parse().map_err(E::custom).and_then(|v| self.visit_u64(v))
    }
}
