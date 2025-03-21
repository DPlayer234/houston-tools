use std::collections::HashSet;
use std::hash::Hash;

use serenity::futures::future::{BoxFuture, always_ready};

pub mod bson;
pub mod discord;
pub mod index_extract_map;
pub mod time;

/// Returns a ZST boxed future that does nothing.
pub fn noop_future() -> BoxFuture<'static, ()> {
    Box::pin(always_ready(|| {}))
}

pub fn is_unique_set<T: Hash + Eq>(iter: impl IntoIterator<Item = T>) -> bool {
    let mut known = HashSet::new();

    for item in iter {
        if !known.insert(item) {
            return false;
        }
    }

    true
}
