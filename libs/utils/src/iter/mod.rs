mod const_iter;
mod vec_chunks;

pub use const_iter::ConstIter;
pub use vec_chunks::VecChunks;

pub trait IteratorExt: Iterator {
    /// Adapts this iterator to yield items in chunks as [`Vec<T>`].
    ///
    /// Panics if `chunk_size` is 0.
    fn vec_chunks(self, chunk_size: usize) -> VecChunks<Self>
    where
        Self: Sized,
    {
        VecChunks::new(self, chunk_size)
    }
}

impl<I: ?Sized> IteratorExt for I where I: Iterator {}
