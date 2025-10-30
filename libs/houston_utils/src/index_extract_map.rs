//! Combines [`extract_map`] and [`indexmap`] through a set and some wrappers.
//!
//! Currently only used for some configuration values, so it has only a small
//! set of needed functions and can only be constructed by deserialization.

use std::hash::{BuildHasher, Hash, Hasher, RandomState};
use std::marker::PhantomData;
use std::{fmt, mem};

use extract_map::ExtractKey;
use indexmap::{Equivalent, IndexSet};
use serde::Deserialize;
use serde::de::{Deserializer, Error as _, SeqAccess, Visitor};

/// Insert-order preserving map whose values store their own keys, exposed via
/// [`ExtractKey`].
#[derive(Debug, Clone)]
pub struct IndexExtractMap<K, V, S = RandomState> {
    inner: IndexSet<Value<K, V>, S>,
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

impl<K, V> Hash for Value<K, V>
where
    K: Hash + Eq,
    V: ExtractKey<K>,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.extract_key().hash(state);
    }
}

impl<K, V> PartialEq for Value<K, V>
where
    K: Hash + Eq,
    V: ExtractKey<K>,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.extract_key() == other.0.extract_key()
    }
}

impl<K, V> Eq for Value<K, V>
where
    K: Hash + Eq,
    V: ExtractKey<K>,
{
}

/// Transparent new-type wrapper around a key.
///
/// Needed to implement [`Equivalent`] in terms of [`ExtractKey`].
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[repr(transparent)]
struct Key<K: ?Sized>(K);

impl<K: ?Sized> Key<K> {
    /// Turns a reference to `K` into one to [`Key<K>`].
    fn from_ref(v: &K) -> &Self {
        // SAFETY: `Key<K>` is a transparent wrapper around `K`
        unsafe { mem::transmute::<&K, &Self>(v) }
    }
}

impl<Q, K, V> Equivalent<Value<K, V>> for Key<Q>
where
    Q: ?Sized + Hash + Equivalent<K>,
    K: Hash + Eq,
    V: ExtractKey<K>,
{
    fn equivalent(&self, key: &Value<K, V>) -> bool {
        self.0.equivalent(key.0.extract_key())
    }
}

impl<K, V, S> IndexExtractMap<K, V, S> {
    /// Gets the number of the elements in the map.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Gets whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns an iterator to the values of the map, in their order.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.inner.iter().map(Value::get_ref)
    }
}

// minimal set of functions i need
impl<K, V, S> IndexExtractMap<K, V, S>
where
    K: Hash + Eq,
    V: ExtractKey<K>,
    S: BuildHasher,
{
    /// Gets a reference to the value stored in the set, if it is present, else
    /// [`None`].
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        self.inner.get(Key::from_ref(key)).map(Value::get_ref)
    }

    /// Returns an iterator to the keys of the map, in their order.
    ///
    /// This is equivalent to: `map.values().map(|v| v.extract_key())`
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.inner.iter().map(|h| h.0.extract_key())
    }
}

impl<'de, K, V, S> Deserialize<'de> for IndexExtractMap<K, V, S>
where
    K: Hash + Eq,
    V: ExtractKey<K> + Deserialize<'de>,
    S: BuildHasher + Default,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MapVisitor<K, V, S>(PhantomData<IndexExtractMap<K, V, S>>);

        impl<'de, K, V, S> Visitor<'de> for MapVisitor<K, V, S>
        where
            K: Hash + Eq,
            V: ExtractKey<K> + Deserialize<'de>,
            S: BuildHasher + Default,
        {
            type Value = IndexExtractMap<K, V, S>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("sequence of keyed values")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let cap = size_hint_cautious::<V>(seq.size_hint());
                let mut map = IndexSet::with_capacity_and_hasher(cap, S::default());
                while let Some(item) = seq.next_element::<V>()? {
                    if !map.insert(Value::new(item)) {
                        return Err(A::Error::custom("duplicate key in sequence"));
                    }
                }

                map.shrink_to_fit();
                Ok(IndexExtractMap { inner: map })
            }
        }

        deserializer.deserialize_seq(MapVisitor(PhantomData))
    }
}

// taken from how serde deals with size hints also
fn size_hint_cautious<T>(hint: Option<usize>) -> usize {
    // basically allocate only up to 1 MB upfront
    const MAX: usize = 1024 * 1024;
    MAX.checked_div(size_of::<T>()).min(hint).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::hash::{BuildHasher as _, RandomState};

    use extract_map::ExtractKey;
    use indexmap::Equivalent as _;

    use super::{IndexExtractMap, Key, Value};

    #[derive(Debug, Clone, PartialEq)]
    struct Item {
        name: String,
        value: i32,
    }

    impl ExtractKey<String> for Item {
        fn extract_key(&self) -> &String {
            &self.name
        }
    }

    #[test]
    fn key_equality() {
        let key_value = "alice";

        let item = Item {
            name: key_value.to_owned(),
            value: 32,
        };

        let value = Value::new(item);
        let key = Key::from_ref(value.get_ref().extract_key());

        let hasher = RandomState::new();
        let hash = hasher.hash_one(&value);

        assert!(key.equivalent(&value), "{key:?}.equivalent(&{value:?})");
        assert_eq!(hasher.hash_one(key), hash);

        assert!(
            Key::from_ref(key_value).equivalent(&value),
            "Key::from_ref({key_value:?}).equivalent(&{value:?})"
        );
        assert_eq!(hasher.hash_one(Key::from_ref(key_value)), hash);
    }

    #[test]
    fn map_entries() {
        let alice = Item {
            name: "alice".to_owned(),
            value: 32,
        };
        let bob = Item {
            name: "bob".to_owned(),
            value: 28,
        };

        let map = IndexExtractMap {
            inner: indexmap::indexset! {
                Value::new(alice.clone()),
                Value::new(bob.clone()),
            },
        };

        assert_eq!(map.get("alice"), Some(&alice));
        assert_eq!(map.get("bob"), Some(&bob));

        assert_eq!(map.get(&alice.name), Some(&alice));
        assert_eq!(map.get(&bob.name), Some(&bob));
    }
}
