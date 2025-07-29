use std::collections::HashSet;
use std::hash::Hash;

pub mod bson;
pub mod discord;
pub mod futures;
pub mod index_extract_map;
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

/// Checks whether the `haystack` contains the `needle` with ASCII
/// case-insensitive comparison.
pub fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }

    let mut haystack = haystack.as_bytes();
    let needle = needle.as_bytes();
    let len = needle.len();

    while haystack.len() >= len {
        // check if the new haystack part starts with the needle
        if haystack[..len].eq_ignore_ascii_case(needle) {
            return true;
        }

        // advance the starting position
        haystack = &haystack[1..];
    }

    false
}
