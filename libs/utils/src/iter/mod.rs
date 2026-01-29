//! Iterator convenience utilities.

mod collect_chunks;
mod const_iter;

pub use collect_chunks::CollectChunks;
pub use const_iter::ConstIter;

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
}

impl<I: ?Sized> IteratorExt for I where I: Iterator {}
