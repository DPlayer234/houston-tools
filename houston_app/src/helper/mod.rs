use std::collections::HashSet;
use std::hash::Hash;

pub mod discord;
pub mod future;
pub mod time;

macro_rules! bson_id {
    ($expr:expr) => {{
        #[allow(clippy::cast_possible_wrap)]
        let value = $expr.get() as i64;
        value
    }};
}

pub(crate) use bson_id;

pub fn is_unique_set<T: Hash + Eq>(iter: impl IntoIterator<Item = T>) -> bool {
    let mut known = HashSet::new();

    for item in iter {
        if !known.insert(item) {
            return false;
        }
    }

    true
}
