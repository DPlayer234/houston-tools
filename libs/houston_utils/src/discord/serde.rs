use serde::de::Error as _;
use serde::{Deserialize as _, Deserializer, Serialize as _, Serializer};
use serde_with::{DeserializeAs, SerializeAs};

/// Marker type to use with [`serde_with`] in place of the ID type to
/// serialize Discord IDs as fixed-length 8-byte values.
pub enum IdBytes {}

// LEB128 isn't really efficient for Discord IDs so circumvent that by encoding
// them as byte arrays. we also need an override anyways because serenity tries
// to deserialize them as any and that's no good.
trait CastU64: From<u64> + Into<u64> + Copy {}
impl<T: From<u64> + Into<u64> + Copy> CastU64 for T {}

impl<T: CastU64> SerializeAs<T> for IdBytes {
    fn serialize_as<S>(source: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let val = *source;
        let int: u64 = val.into();
        int.to_le_bytes().serialize(serializer)
    }
}

impl<'de, T: CastU64> DeserializeAs<'de, T> for IdBytes {
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
    {
        // serenity's ids panic if you try to construct them from `u64::MAX`
        // since they are backed by `NonMaxU64` inner values.
        let int = <[u8; 8]>::deserialize(deserializer)?;
        let int = u64::from_le_bytes(int);
        if int != u64::MAX {
            Ok(T::from(int))
        } else {
            Err(D::Error::custom("discord id cannot be u64::MAX"))
        }
    }
}
