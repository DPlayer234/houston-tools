//! Provides utility types for handling compatability.

use serde_core::de::Error as _;
use serde_core::{Deserialize, Deserializer, Serialize, Serializer};

/// A marker type to reliably reject older or newer versions of the structure on
/// deserialization.
///
/// Add a field of this type to your struct, starting with [`VersionTag<0>`],
/// ideally as the first field. When you make format-incompatible changes to the
/// struct, increment its const-generic.
///
/// If you have other means to support changes in data format, this type is
/// redundant.
///
/// It serializes as its const-generic. On deserialization, verifies that the
/// correct value is present and otherwise fails deserialization.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionTag<const V: u64>;

impl<const V: u64> Serialize for VersionTag<V> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        V.serialize(serializer)
    }
}

impl<'de, const V: u64> Deserialize<'de> for VersionTag<V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = u64::deserialize(deserializer)?;
        if v == V {
            Ok(Self)
        } else {
            Err(D::Error::custom(format!(
                "version check failed; got: {v}, expected: {V}"
            )))
        }
    }
}
