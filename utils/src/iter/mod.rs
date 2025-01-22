mod vec_chunks;

pub use vec_chunks::VecChunks;

pub trait IteratorExt: Iterator {
    /// Adapts this iterator to yield items in chunks as [`Vec<T>`].
    ///
    /// Panics if `size` is 0.
    fn vec_chunks(self, size: usize) -> VecChunks<Self>
    where
        Self: Sized,
    {
        VecChunks::new(self, size)
    }
}

impl<I: ?Sized> IteratorExt for I where I: Iterator {}
