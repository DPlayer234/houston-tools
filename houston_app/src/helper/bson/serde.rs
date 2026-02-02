//! Helper functions for serializing values in BSON.
//!
//! This primarily deals with serializing [serenity]'s IDs as integers.

use std::marker::PhantomData;

use serde::de::{Error, Visitor};
use serde::{Deserialize as _, Deserializer, Serialize as _, Serializer};
use serde_with::{DeserializeAs, SerializeAs};
use time::UtcDateTime;

use crate::helper::discord::CastU64;

/// Marker type to use with [`serde_with`] in place of the ID type to serialize
/// the Discord ID's underlying [`u64`] value as [`i64`], and reversing
/// that process for deserialization.
///
/// When deserializing, also accepts stringified [`u64`] values.
pub enum IdBson {}

impl<T: CastU64> SerializeAs<T> for IdBson {
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let int: u64 = (*source).into();
        serializer.serialize_i64(int.cast_signed())
    }
}

impl<'de, T: CastU64> DeserializeAs<'de, T> for IdBson {
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(I64Visitor(PhantomData))
    }
}

/// Helper to accept either [`i64`] or [`str`].
struct I64Visitor<T>(PhantomData<T>);

impl<T: CastU64> Visitor<'_> for I64Visitor<T> {
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

pub enum DateTimeBson {}

impl SerializeAs<UtcDateTime> for DateTimeBson {
    fn serialize_as<S>(source: &UtcDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        bson::DateTime::from_time_0_3((*source).into()).serialize(serializer)
    }
}

impl<'de> DeserializeAs<'de, UtcDateTime> for DateTimeBson {
    fn deserialize_as<D>(deserializer: D) -> Result<UtcDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(bson::DateTime::deserialize(deserializer)?
            .to_time_0_3()
            .to_utc())
    }
}

#[cfg(test)]
mod tests {
    use bson::Bson;
    use bson::de::Deserializer;
    use bson::ser::Serializer;
    use bson::serde_helpers::datetime::FromTime03OffsetDateTime;
    use serde_with::As;
    use serenity::model::id::UserId;
    use time::OffsetDateTime;
    use time::macros::{datetime, utc_datetime};

    use super::*;

    #[test]
    fn from_i64() {
        let bson = Bson::Int64(1234);
        let de = Deserializer::new(bson);
        let id: UserId = As::<IdBson>::deserialize(de).expect("must deserialize");

        assert_eq!(id, UserId::new(1234));
    }

    #[test]
    fn from_str() {
        let bson = Bson::String("2345".to_owned());
        let de = Deserializer::new(bson);
        let id: UserId = As::<IdBson>::deserialize(de).expect("must deserialize");

        assert_eq!(id, UserId::new(2345));
    }

    #[test]
    fn datetime_same_as_builtin_helper1() {
        let origin_datetime = datetime!(2025-04-01 15:42:31+02);

        let ser = Serializer::new();
        let bson = As::<FromTime03OffsetDateTime>::serialize(&origin_datetime, ser)
            .expect("must serialize");

        let de = Deserializer::new(bson);
        let output_datetime: UtcDateTime =
            As::<DateTimeBson>::deserialize(de).expect("must deserialize");

        assert_eq!(output_datetime, origin_datetime.to_utc());
    }

    #[test]
    fn datetime_same_as_builtin_helper2() {
        let origin_datetime = utc_datetime!(2025-04-02 15:42:31);

        let ser = Serializer::new();
        let bson = As::<DateTimeBson>::serialize(&origin_datetime, ser).expect("must serialize");

        let de = Deserializer::new(bson);
        let output_datetime: OffsetDateTime =
            As::<FromTime03OffsetDateTime>::deserialize(de).expect("must deserialize");

        assert_eq!(output_datetime.to_utc(), origin_datetime);
    }

    // this does not check for `Ok(_)`, but just that there isn't a panic
    fn no_panic_on_de(val: i64) {
        let ser = Serializer::new();
        let bson = As::<IdBson>::serialize(&val.cast_unsigned(), ser).expect("must serialize");
        let de = Deserializer::new(bson);
        let _: Result<UserId, _> = As::<IdBson>::deserialize(de);
    }

    #[test]
    fn edge_values_ok() {
        no_panic_on_de(0);
        no_panic_on_de(-1);
        no_panic_on_de(i64::MIN);
        no_panic_on_de(i64::MAX);
    }
}
