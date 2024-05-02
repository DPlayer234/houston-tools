//! Contains the [`PrefixMap`] struct.

/// Provides a way to map values of type `T` to strings and access all values whose key starts with a certain prefix.
/// 
/// The access here is at least O(log n). The map is backed by a vector which is binary searched.
#[derive(Debug, Clone)]
pub struct PrefixMap<T> {
    map: Vec<Entry<T>>
}

/// A key-value pair for [`PrefixMap`].
#[derive(Debug, Clone)]
struct Entry<T> {
    key: String,
    value: T
}

impl<T> PrefixMap<T> {
    /// Creates a new empty map.
    pub fn new() -> Self {
        Self { map: Vec::new() }
    }

    /// Inserts a new value into the map.
    /// 
    /// Insertion is O(n log n).
    pub fn insert(&mut self, key: &str, value: T) -> bool {
        let key = simplify(key);
        match self.search_by(&key) {
            Ok(_) => false,
            Err(index) => { self.map.insert(index, Entry { key, value }); true }
        }
    }

    /// Provides an iterator over all entries whose key starts with the provided prefix.
    /// 
    /// This function in itself is O(log n), looking for the start of the data.
    /// The data is evaluated lazily as you use the iterator.
    pub fn find(&self, key_prefix: &str) -> impl Iterator<Item = &T> {
        let key_prefix = simplify(key_prefix);
        let start = self.search_by(&key_prefix).unwrap_or_else(std::convert::identity);

        self.map[start..].iter()
            .take_while(move |e| e.key.starts_with(&key_prefix))
            .map(|e| &e.value)
    }

    fn search_by(&self, key: &str) -> Result<usize, usize> {
        self.map.binary_search_by_key(&key, |e| e.key.as_str())
    }
}

fn is_allowed_char(c: &char) -> bool {
    c.is_alphanumeric()
}

fn simplify(key: &str) -> String {
    key.chars().filter(is_allowed_char).filter_map(|c| c.to_lowercase().next()).collect()
}
