use std::fmt;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use small_fixed_array::{FixedArray, ValidLength as _};

pub trait ThinBMapKey: Copy + Ord {
    /// The total count of known keys.
    const COUNT: usize;

    /// Gets the stringified name of the active variant.
    ///
    /// This matches the name of the field this corresponds to in the
    /// serialized format.
    #[must_use]
    fn name(self) -> &'static str;
}

/// A "thin" binary map.
///
/// This is optimized for memory rather than access speed.
#[derive(Clone)]
pub struct ThinBMap<K, V>(
    /// A list of key-value pairs sorted by the key.
    FixedArray<(K, V)>,
);

// no way offered to "unwrap" this struct since the assumption is that it's
// mostly used for borrowed data and rarely in an owned consumable form.
impl<K: ThinBMapKey, V> ThinBMap<K, V> {
    fn key_fn(t: &(K, V)) -> K {
        t.0
    }

    /// Creates a new skin words map.
    ///
    /// This array is sorted by the key upon construction and does not need to
    /// be pre-sorted.
    ///
    /// # Errors
    ///
    /// Returns `Err` when a key is duplicated.
    pub fn new(mut value: FixedArray<(K, V)>) -> Result<Self, ThinBMapError<K>> {
        value.sort_unstable_by_key(Self::key_fn);

        // ensure there are no duplicate keys provided. since it's already sorted by the
        // keys, comparing all pairs of adjacent keys is good enough to figure that out.
        for window in value.windows(2) {
            let [l, r] = window.as_array().expect("must be len 2");
            if l.0 == r.0 {
                return Err(ThinBMapError::DuplicateKey(l.0));
            }
        }

        Ok(Self(value))
    }

    /// The amount of lines stored.
    pub fn len(&self) -> usize {
        self.0.len().to_usize()
    }

    /// Whether this collection is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Gets the line for a specific key, if present.
    pub fn get(&self, key: K) -> Option<&V> {
        let slice = self.0.as_slice();
        let index = slice.binary_search_by_key(&key, Self::key_fn).ok()?;
        Some(&slice[index].1)
    }

    /// Iterates over all key-value pairs.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (K, &V)> + ExactSizeIterator {
        self.0.iter().map(|(key, value)| (*key, value))
    }
}

// debug and serde as a map
impl<K, V> fmt::Debug for ThinBMap<K, V>
where
    K: fmt::Debug + ThinBMapKey,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, V> Serialize for ThinBMap<K, V>
where
    K: Serialize + ThinBMapKey,
    V: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_map(self.iter())
    }
}

impl<'de, K, V> Deserialize<'de> for ThinBMap<K, V>
where
    K: Deserialize<'de> + ThinBMapKey,
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error as _, MapAccess, Visitor};

        struct ThisVisitor<K, V>(PhantomData<[(K, V)]>);

        impl<'de, K, V> Visitor<'de> for ThisVisitor<K, V>
        where
            K: Deserialize<'de> + ThinBMapKey,
            V: Deserialize<'de>,
        {
            type Value = ThinBMap<K, V>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("key-value pairs")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let count = map.size_hint().unwrap_or_default().max(K::COUNT);

                let mut buf = Vec::with_capacity(count);
                while let Some(entry) = map.next_entry::<K, V>()? {
                    buf.push(entry);
                }

                let buf = buf.try_into().map_err(A::Error::custom)?;
                ThinBMap::new(buf)
                    .map_err(|ThinBMapError::DuplicateKey(k)| A::Error::duplicate_field(k.name()))
            }
        }

        deserializer.deserialize_map(ThisVisitor(PhantomData))
    }
}

/// Error when constructing a [`ThinBMap`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ThinBMapError<K> {
    #[error("key {0:?} was duplicated")]
    DuplicateKey(K),
}
