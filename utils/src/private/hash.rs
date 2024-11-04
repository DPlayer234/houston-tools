use std::hash::{DefaultHasher, Hash, Hasher};

/// Convenience method to calculate the hash of a value with the [`DefaultHasher`].
#[must_use]
#[inline]
pub fn hash_default<T: Hash + ?Sized>(value: &T) -> u64 {
    hash(value, DefaultHasher::new())
}

/// Convenience method to feed a value to a hasher and then return its value.
#[must_use]
#[inline]
pub fn hash<T: Hash + ?Sized, H: Hasher>(value: &T, mut hasher: H) -> u64 {
    value.hash(&mut hasher);
    hasher.finish()
}
