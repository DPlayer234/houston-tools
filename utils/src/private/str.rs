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

#[cfg(test)]
mod tests {
    // needed for test but unused in benchmark
    #[allow(unused_imports)]
    use super::Indices;

    #[allow(dead_code)]
    fn assert_eq_indices(s: &str) {
        let indices: Vec<usize> = Indices::new(s).collect();
        let expected: Vec<usize> = s.char_indices().map(|(i, _)| i).collect();
        assert_eq!(indices, expected);
    }

    #[test]
    fn indices_empty() {
        assert_eq_indices("");
    }

    #[test]
    fn indices_ascii() {
        assert_eq_indices("hello");
    }

    #[test]
    fn indices_unicode() {
        assert_eq_indices("héllo");
    }

    #[test]
    fn indices_emoji() {
        assert_eq_indices("hello😊");
    }

    #[test]
    fn indices_mixed() {
        assert_eq_indices("héllo😊");
    }

    #[test]
    fn nth() {
        let s = "héllo😊";
        for i in 0..6 {
            assert_eq!(
                Indices::new(s).nth(i),
                s.char_indices().nth(i).map(|(i, _)| i),
                "mismatch at index {i}",
            );
        }
    }

    #[test]
    fn count() {
        let s = "héllo😊";
        let indices = Indices::new(s);
        assert_eq!(indices.count(), s.chars().count());
    }

    #[test]
    fn size_hint() {
        let s = "héllo😊";
        let indices = Indices::new(s);
        assert_eq!(indices.size_hint(), s.chars().size_hint());
    }
}
