// Note: benchmarks pull in this file via `include!` so don't reference other
// modules in this crate without checking that the benchmarks still compile.

use std::str::Chars;

/// Provides just the indices of [`char`]s in a string.
///
/// [`CharIndices`] does not specialize [`Iterator::nth`] even though [`Chars`]
/// does. This makes it pretty slow for seeking to a specific character index.
///
/// This iterator on the other hand properly delegates to [`Chars`].
/// For the use in [`crate::text::truncate`], this reduces the time needed by up
/// ~80%.
///
/// The [`char`]s are omitted from the iterator just because I don't need them
/// for what I do.
///
/// [`CharIndices`]: std::str::CharIndices
#[derive(Debug)]
pub struct Indices<'a> {
    iter: Chars<'a>,
    offset: usize,
}

impl<'a> Indices<'a> {
    /// Creates a new iterator over the char indices in `str`.
    pub fn new(str: &'a str) -> Self {
        Self {
            iter: str.chars(),
            offset: 0,
        }
    }
}

impl Iterator for Indices<'_> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let pre_len = self.iter.as_str().len();
        let c = self.iter.next()?;

        self.offset += pre_len - self.iter.as_str().len();
        Some(self.offset - c.len_utf8())
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let pre_len = self.iter.as_str().len();
        let c = self.iter.nth(n)?;

        self.offset += pre_len - self.iter.as_str().len();
        Some(self.offset - c.len_utf8())
    }

    #[inline]
    fn count(self) -> usize {
        self.iter.count()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
