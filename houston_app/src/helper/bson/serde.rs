//! Helper functions for serializing values in BSON.
//!
//! This primarily deals with serializing [serenity]'s IDs as integers.

use std::marker::PhantomData;

use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Serializes `T`'s underlying [`u64`] value as [`i64`], and reverses that
/// process for deserialization.
///
/// When deserializing, also accepts stringified [`u64`] values.
#[repr(transparent)]
struct I64<T>(T);

impl<T: CastI64> I64<T> {
    fn from_u64(u: u64) -> Option<Self> {
        // serenity's ids panic if you try to construct them from `u64::MAX`
        // since they are backed by `NonMaxU64` inner values.
        if u != u64::MAX {
            Some(Self(T::from(u)))
        } else {
            None
        }
    }
}

trait CastI64: From<u64> + Into<i64> + Copy {}
impl<T: From<u64> + Into<i64> + Copy> CastI64 for T {}

impl<T: CastI64> Serialize for I64<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let int = self.0.into();
        serializer.serialize_i64(int)
    }
}

impl<'de, T: CastI64> Deserialize<'de> for I64<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(I64Visitor(PhantomData))
    }
}

struct I64Visitor<T>(PhantomData<T>);

impl<T: CastI64> Visitor<'_> for I64Visitor<T> {
    type Value = I64<T>;

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
        I64::from_u64(v).ok_or_else(|| E::custom("discord id cannot be u64::MAX"))
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

/// Serializes a Discord ID as an [`i64`].
#[expect(private_bounds)]
pub mod id_as_i64 {
    use super::*;

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: CastI64,
    {
        I64::deserialize(deserializer).map(|i| i.0)
    }

    pub fn serialize<S, T>(val: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: CastI64,
    {
        I64(*val).serialize(serializer)
    }
}

/// Serializes a collection of Discord IDs as a sequence of [`i64`].
#[expect(private_bounds)]
pub mod coll_id_as_i64 {
    use super::*;

    // in all honesty, i'm impressed that it can somehow infer the generics when
    // this function is called by serde's derive macro
    pub fn deserialize<'de, D, T, C>(deserializer: D) -> Result<C, D::Error>
    where
        D: Deserializer<'de>,
        T: CastI64,
        C: FromIterator<T>,
    {
        let vec = <Vec<I64<T>>>::deserialize(deserializer)?;

        // if `C` is `Vec<T>`, this collect should use in-place collection, and optimize
        // away fully due to `Id<T>` being transparent over `T`. if it doesn't
        // optimize anyways, that's fine too. this code isn't perf sensitive.
        Ok(vec.into_iter().map(|item| item.0).collect())
    }

    pub fn serialize<S, T, C>(val: &C, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: CastI64,
        // weird bound, but since `C` is intended to be f.e. `Vec<T>`, this is what we'd usually
        // expect this to be called with and leads to the least friction.
        for<'a> &'a C: IntoIterator<Item = &'a T>,
    {
        serializer.collect_seq(val.into_iter().map(|t| I64(*t)))
    }
}
