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

macro_rules! doc_object_id {
    ($expr:expr) => {
        #[allow(clippy::used_underscore_binding)]
        {
            ::bson::doc! {
                "_id": $expr._id
            }
        }
    };
}

pub(crate) use bson_id;
pub(crate) use doc_object_id;

pub fn is_unique_set<T: Hash + Eq>(iter: impl IntoIterator<Item = T>) -> bool {
    let mut known = HashSet::new();

    for item in iter {
        if !known.insert(item) {
            return false;
        }
    }

    true
}
