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
        self.map[start..].iter()
            .take_while(move |e| e.key.starts_with(&key))
            .map(|e| &e.value)
    }
}

fn is_allowed_char(c: &char) -> bool {
    c.is_alphanumeric()
}

fn simplify(key: &str) -> String {
    key.chars().filter(is_allowed_char).filter_map(|c| c.to_lowercase().next()).collect()
}
