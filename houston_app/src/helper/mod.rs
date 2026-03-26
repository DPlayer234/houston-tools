use std::collections::HashSet;
use std::hash::Hash;
use std::mem;

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

/// Allows easily making sure to only enter a branch once.
#[derive(Debug, Default)]
pub struct BranchOnce {
    entered: bool,
}

impl BranchOnce {
    /// Creates a new un-entered [`BranchOnce`].
    pub const fn new() -> Self {
        Self { entered: false }
    }

    /// Tries to enter the branch.
    ///
    /// Returns `true` if this is the first time this method has been called on
    /// this instance, otherwise `false`.
    pub const fn enter(&mut self) -> bool {
        !mem::replace(&mut self.entered, true)
    }
}
