use std::collections::HashSet;
use std::hash::Hash;

pub mod bson;
pub mod discord;
pub mod future;
pub mod sync;
pub mod time;

pub fn is_unique_set<T: Hash + Eq>(iter: impl IntoIterator<Item = T>) -> bool {
    let mut known = HashSet::new();

    for item in iter {
        if !known.insert(item) {
            return false;
        }
    }

    true
}
