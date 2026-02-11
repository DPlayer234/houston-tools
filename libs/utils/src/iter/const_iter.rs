use std::mem::replace;

/// Iterator-like for slices that is const-compatible.
///
/// This works for both immutable and mutable slices the same way. If you leave
/// the iteration before the end, the remaining slice can be obtained with
/// [`Self::into_slice`].
#[must_use = "ConstIter does nothing unless used"]
pub struct ConstIter<S> {
    slice: S,
}

impl<S> ConstIter<S> {
    /// Constructs a new const-compatible iterator over a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use utils::iter::ConstIter;
    /// # let slice: &[i32] = &[1, 2, 3];
    /// # fn do_something(_: &i32) {}
    /// # _ = stringify! {
    /// let slice: &[i32] = ...;
    /// # };
    ///
    /// let mut iter = ConstIter::new(slice);
    /// while let Some(item) = iter.next() {
    ///     do_something(item);
    /// }
    /// ```
    pub const fn new(slice: S) -> Self {
        Self { slice }
    }
}

impl<'a, T> ConstIter<&'a [T]> {
    /// Gets a reference to the next slice item.
    ///
    /// Returns [`None`] if the iterator is exhausted.
    pub const fn next(&mut self) -> Option<&'a T> {
        match self.slice {
            [next, rest @ ..] => {
                self.slice = rest;
                Some(next)
            },
            _ => None,
        }
    }

    /// Gets a reference to the next slice item from the back.
    ///
    /// Returns [`None`] if the iterator is exhausted.
    pub const fn next_back(&mut self) -> Option<&'a T> {
        match self.slice {
            [rest @ .., next] => {
                self.slice = rest;
                Some(next)
            },
            _ => None,
        }
    }

    /// Gets the remaining slice.
    pub const fn into_slice(self) -> &'a [T] {
        self.slice
    }
}

#[expect(
    clippy::mem_replace_with_default,
    reason = "cannot use mem::take in const"
)]
impl<'a, T> ConstIter<&'a mut [T]> {
    /// Gets a reference to the next slice item.
    ///
    /// Returns [`None`] if the iterator is exhausted.
    pub const fn next(&mut self) -> Option<&'a mut T> {
        // need this replace here so the lifetimes work out
        match replace(&mut self.slice, &mut []) {
            [next, rest @ ..] => {
                self.slice = rest;
                Some(next)
            },
            _ => None,
        }
    }

    /// Gets a reference to the next slice item from the back.
    ///
    /// Returns [`None`] if the iterator is exhausted.
    pub const fn next_back(&mut self) -> Option<&'a mut T> {
        // need this replace here so the lifetimes work out
        match replace(&mut self.slice, &mut []) {
            [rest @ .., next] => {
                self.slice = rest;
                Some(next)
            },
            _ => None,
        }
    }

    /// Gets the remaining slice.
    pub const fn into_slice(self) -> &'a mut [T] {
        self.slice
    }
}
