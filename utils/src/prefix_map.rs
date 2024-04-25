use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PrefixMap<T> {
    map: Vec<Entry<T>>
}

#[derive(Debug, Clone)]
struct Entry<T> {
    key: Arc<str>,
    value: T
}

impl<T> PrefixMap<T> {
    pub fn new() -> Self {
        Self { map: Vec::new() }
    }

    pub fn insert(&mut self, key: &str, value: T) {
        let key = simplify(key);
        let index = self.map.binary_search_by_key(&key.as_str(), |e| &*e.key);
        let entry = Entry { key: Arc::from(key), value };
        match index {
            Ok(_) => (),
            Err(index) => self.map.insert(index, entry)
        };
    }

    pub fn find(&self, key: &str) -> impl Iterator<Item = &T> {
        let key = simplify(key);
        let start = self.map.binary_search_by_key(&key.as_str(), |e| &*e.key).unwrap_or_else(std::convert::identity);

        let mut index = start;
        while index < self.map.len() && self.map[index].key.starts_with(&key) {
            index += 1;
        }

        self.map[start..index].iter().map(|e| &e.value)
    }
}

fn simplify(key: &str) -> String {
    key.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect()
}
