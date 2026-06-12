//! Iterator convenience utilities.

mod collect_chunks;
mod const_iter;
mod runs;

pub use collect_chunks::CollectChunks;
pub use const_iter::ConstIter;
pub use runs::{Runs, RunsBy, RunsByKey};

/// Iterates over chunks of values, each yielded as a [`Vec`].
pub type VecChunks<I> = CollectChunks<I, Vec<<I as Iterator>::Item>>;

/// Extension trait for all [`Iterator`] types.
pub trait IteratorExt: Iterator {
    /// Adapts this iterator to yield items in chunks as [`Vec<T>`].
    ///
    /// This is equivalent to [`IteratorExt::collect_chunks`] with [`Vec`] as
    /// `C`.
    ///
    /// # Panics
    ///
    /// Panics if `chunk_size` is 0.
    fn vec_chunks(self, chunk_size: usize) -> VecChunks<Self>
    where
        Self: Sized,
    {
        VecChunks::new(self, chunk_size)
    }

    /// Adapts this iterator to yield items in chunks as the collection type
    /// specified by `C`. This supports any [`FromIterator`] type, and functions
    /// as if calling [`Iterator::collect`] on sub-sections of the source
    /// iterator.
    ///
    /// # Notes
    ///
    /// Much like [`Iterator::collect`], this supports a couple of unusual
    /// operations, like collecting chunks of [`char`] into [`String`].
    ///
    /// [`DoubleEndedIterator`] attempts to be consistent with `last` and is
    /// therefore only implemented for [`ExactSizeIterator`] and if `C` is
    /// [`AsMut<[I::Item]>`][AsMut] due to implementation constraints. An
    /// incorrectly implemented [`ExactSizeIterator`] implementation for `Self`
    /// may lead to buggy iteration-from-back behavior.
    ///
    /// If [`C as FromIterator`][FromIterator] does not fully consume the
    /// provided iterator, this function will drain the rest to ensure the
    /// chunks are split as expected. This affects implementations such as
    /// [`Option<C>`], which short-circuit.
    ///
    /// # Panics
    ///
    /// Panics if `chunk_size` is 0.
    fn collect_chunks<C>(self, chunk_size: usize) -> CollectChunks<Self, C>
    where
        Self: Sized,
        C: FromIterator<Self::Item>,
    {
        CollectChunks::new(self, chunk_size)
    }

    /// Iterates over runs of equivalent consecutive elements, based on
    /// [`PartialEq`].
    ///
    /// The resulting iterator will yield tuples of `(item, run_length)`.
    ///
    /// If the iterator is sorted and `Self::Item` is [`Eq`], the resulting
    /// iterator returns unique elements.
    ///
    /// This function is equivalent to
    /// [`self.runs_by(PartialEq::eq)`](Self::runs_by).
    fn runs(self) -> Runs<Self>
    where
        Self: Sized,
        Self::Item: PartialEq,
    {
        Runs::new(self)
    }

    /// Iterates over runs of equivalent consecutive elements, with equality
    /// provided through an equality function.
    ///
    /// The resulting iterator will yield tuples of `(item, run_length)`.
    fn runs_by<F>(self, eq: F) -> RunsBy<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Item, &Self::Item) -> bool,
    {
        RunsBy::new(self, eq)
    }

    /// Iterates over runs of equivalent consecutive elements, with equality
    /// provided by comparing a key.
    ///
    /// The resulting iterator will yield tuples of `(item, run_length)`.
    ///
    /// This function is equivalent to
    /// [`self.runs_by(|a, b| f(a) == f(b))`](Self::runs_by).
    fn runs_by_key<F, K>(self, f: F) -> RunsByKey<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Item) -> K,
        K: PartialEq,
    {
        RunsByKey::new(self, f)
    }

    /// Returns the single item in the iterator.
    ///
    /// Returns `None` if the iterator is empty or has more than one item.
    ///
    /// # Notes
    ///
    /// This may advance the iterator either once or twice. It attempts to use
    /// the [`Iterator::size_hint`] to avoid yielding a second item, but
    /// imprecise hints may not allow deriving whether the iterator is empty
    /// after the first item.
    fn single(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let result = self.next()?;
        match self.size_hint() {
            // iterator reports that it yields a maximum of 0 more elements, so it is empty now and
            // the item we already got is the single item that we should be returning here.
            (_, Some(0)) => Some(result),
            // iterator reports it yields at least 1 more element, so it can't be empty.
            (1.., _) => None,
            // other size hints, i.e. `(0, any)`, don't provide the enough info. get another item.
            _ => match self.next() {
                None => Some(result),
                Some(_) => None,
            },
        }
    }
}

impl<I: ?Sized> IteratorExt for I where I: Iterator {}

#[cfg(test)]
mod tests {
    use super::IteratorExt as _;

    /// Iterator adapter that suppresses optimization opportunities.
    struct Unknown<I>(I);

    impl<I: Iterator> Iterator for Unknown<I> {
        type Item = I::Item;

        fn next(&mut self) -> Option<Self::Item> {
            self.0.next()
        }
    }

    #[test]
    fn single_success_size_hint() {
        let iter = { &[42] }.iter();
        assert_eq!(iter.single(), Some(&42));
    }

    #[test]
    fn single_fail_empty_size_hint() {
        let iter = { &[] }.iter();
        assert_eq!(iter.single(), None::<&i32>);
    }

    #[test]
    fn single_fail_too_long1_size_hint() {
        let iter = { &[1, 2] }.iter();
        assert_eq!(iter.single(), None::<&i32>);
    }

    #[test]
    fn single_fail_too_long2_size_hint() {
        let iter = { &[1, 2, 3] }.iter();
        assert_eq!(iter.single(), None::<&i32>);
    }

    #[test]
    fn single_success() {
        let iter = Unknown({ &[42] }.iter());
        assert_eq!(iter.single(), Some(&42));
    }

    #[test]
    fn single_fail_empty() {
        let iter = Unknown({ &[] }.iter());
        assert_eq!(iter.single(), None::<&i32>);
    }

    #[test]
    fn single_fail_too_long1() {
        let iter = Unknown({ &[1, 2] }.iter());
        assert_eq!(iter.single(), None::<&i32>);
    }

    #[test]
    fn single_fail_too_long2() {
        let iter = Unknown({ &[1, 2, 3] }.iter());
        assert_eq!(iter.single(), None::<&i32>);
    }

    #[test]
    fn single_fail_infinite() {
        let iter = std::iter::repeat(99);
        assert_eq!(iter.single(), None::<i32>);
    }

    #[test]
    fn single_fail_infinite_unknown() {
        let iter = Unknown(std::iter::repeat(99));
        assert_eq!(iter.single(), None::<i32>);
    }
}
