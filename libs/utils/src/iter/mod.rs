//! Iterator convenience utilities.

mod collect_chunks;
mod const_iter;
mod runs;

pub use collect_chunks::CollectChunks;
pub use const_iter::ConstIter;
pub use runs::{Runs, RunsBy};

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

    /// Iterates over runs of consecutive elements that are equal based on
    /// [`PartialEq`].
    ///
    /// If the iterator is sorted and `Self::Item` is [`Eq`], the resulting
    /// iterator returns only unique elements.
    ///
    /// This function is equivalent to
    /// [`self.runs_by(PartialEq::eq)`](Self::runs_by).
    fn runs<F>(self) -> Runs<Self>
    where
        Self: Sized,
        Self::Item: PartialEq,
    {
        Runs::new(self)
    }

    /// Iterates over runs of consecutive elements that are equal based on the
    /// provided equality function.
    ///
    /// If the iterator is sorted and the equality matches the contract for
    /// [`Eq`], the resulting iterator returns only unique elements.
    fn runs_by<F>(self, eq_by: F) -> RunsBy<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Item, &Self::Item) -> bool,
    {
        RunsBy::new(self, eq_by)
    }
}

impl<I: ?Sized> IteratorExt for I where I: Iterator {}
