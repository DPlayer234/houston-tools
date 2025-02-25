//! Combines [`extract_map`] and [`indexmap`] through a set and some wrappers.
//!
//! Currently only used for some configuration values, so it has only a small
//! set of needed functions and can only be constructed by deserialization.

use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::{fmt, mem};

use extract_map::ExtractKey;
use indexmap::{Equivalent, IndexSet};
use serde::Deserialize;
use serde::de::{Deserializer, Error as _, SeqAccess, Visitor};

/// Insert-order preserving map whose values store their own keys.
#[derive(Debug, Clone)]
pub struct IndexExtractMap<K, V> {
    inner: IndexSet<Value<K, V>>,
}

/// Transparent wrapper around a value together with the intended key type.
///
/// Needed to reimplement equality traits in terms of `K`.
#[derive(Debug, Clone)]
#[repr(transparent)]
struct Value<K, V>(V, PhantomData<K>);

impl<K, V> Value<K, V> {
    /// Creates a new wrapped value.
    fn new(v: V) -> Self {
        Self(v, PhantomData)
    }

    /// Gets a reference to the inner value.
    fn get_ref(&self) -> &V {
        &self.0
    }
}

impl<K: Hash + Eq, V: ExtractKey<K>> Hash for Value<K, V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.extract_key().hash(state);
    }
}

impl<K: Hash + Eq, V: ExtractKey<K>> PartialEq for Value<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.0.extract_key() == other.0.extract_key()
    }
}

impl<K: Hash + Eq, V: ExtractKey<K>> Eq for Value<K, V> {}

/// Transparent new-type wrapper around a key.
///
/// Needed to implement [`Equivalent`] in terms of [`ExtractKey`].
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[repr(transparent)]
struct Key<K>(K);

impl<K> Key<K> {
    /// Turns a reference to `K` into one to [`Key<K>`].
    fn from_ref(v: &K) -> &Self {
        // SAFETY: `Key<V>` is a transparent wrapper around `V`
        unsafe { mem::transmute::<&K, &Self>(v) }
    }
}

impl<K: Hash + Eq, V: ExtractKey<K>> Equivalent<Value<K, V>> for Key<K> {
    fn equivalent(&self, key: &Value<K, V>) -> bool {
        self.0 == *key.0.extract_key()
    }
}

// minimal set of functions i need
impl<K: Hash + Eq, V: ExtractKey<K>> IndexExtractMap<K, V> {
    /// Gets a reference to the value stored in the set, if it is present, else
    /// `None`.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(Key::from_ref(key)).map(Value::get_ref)
    }

    /// Returns an iterator to the values of the map.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.inner.iter().map(Value::get_ref)
    }

    /// Returns an iterator to the keys of the map.
    ///
    /// This is equivalent to: `map.values().map(|v| v.extract_key())`
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.inner.iter().map(|h| h.0.extract_key())
    }
}

impl<'de, K, V> Deserialize<'de> for IndexExtractMap<K, V>
where
    K: Hash + Eq,
    V: ExtractKey<K> + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MapVisitor<K, V>(PhantomData<IndexExtractMap<K, V>>);

        impl<'de, K, V> Visitor<'de> for MapVisitor<K, V>
        where
            K: Hash + Eq,
            V: ExtractKey<K> + Deserialize<'de>,
        {
            type Value = IndexExtractMap<K, V>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("list of starboards")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut map = IndexSet::new();
                while let Some(item) = seq.next_element::<V>()? {
                    if !map.insert(Value::new(item)) {
                        return Err(A::Error::custom("duplicate starboard id"));
                    }
                }

                map.shrink_to_fit();
                Ok(IndexExtractMap { inner: map })
            }
        }

        deserializer.deserialize_seq(MapVisitor(PhantomData))
    }
}
