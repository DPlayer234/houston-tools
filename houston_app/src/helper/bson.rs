macro_rules! bson_id {
    ($expr:expr) => {{
        #[allow(clippy::cast_possible_wrap)]
        let value = $expr.get() as i64;
        value
    }};
}

macro_rules! doc_object_id {
    ($expr:expr) => {
        #[allow(clippy::used_underscore_binding)]
        {
            ::bson::doc! {
                "_id": $expr._id
            }
        }
    };
}

pub(crate) use bson_id;
pub(crate) use doc_object_id;

pub mod id_as_i64 {
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D, T: From<u64>>(
        deserializer: D,
    ) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[allow(clippy::cast_sign_loss)]
        let int = i64::deserialize(deserializer)? as u64;
        if int != u64::MAX {
            Ok(T::from(int))
        } else {
            Err(D::Error::custom("invalid discord id"))
        }
    }

    pub fn serialize<S: Serializer, T: Into<i64> + Copy>(
        val: &T,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let int: i64 = (*val).into();
        int.serialize(serializer)
    }
}
