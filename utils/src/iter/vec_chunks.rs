/// Iterates over chunks of values, yielded as vectors.
pub struct VecChunks<I> {
    size: usize,
    inner: I,
}

impl<I> VecChunks<I> {
    pub(super) fn new(iter: I, size: usize) -> Self {
        assert!(size != 0, "chunk size must not be zero");
        Self { inner: iter, size }
    }
}

impl<I: Iterator> Iterator for VecChunks<I> {
    type Item = Vec<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        // this figures out the correct capacity for the vec if the iterator provides
        // a useful size hint. take never consumes more than size elements.
        let chunk: Vec<I::Item> = self.inner.by_ref().take(self.size).collect();
        (!chunk.is_empty()).then_some(chunk)
    }
}

#[cfg(test)]
mod tests {
    use crate::iter::IteratorExt as _;

    #[test]
    fn vec_chunks() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut chunks = data.into_iter().vec_chunks(3);

        assert_eq!(chunks.next(), Some(vec![1, 2, 3]));
        assert_eq!(chunks.next(), Some(vec![4, 5, 6]));
        assert_eq!(chunks.next(), Some(vec![7, 8, 9]));
        assert_eq!(chunks.next(), Some(vec![10]));
        assert_eq!(chunks.next(), None);
    }

    #[test]
    #[should_panic = "chunk size must not be zero"]
    fn vec_chunks_zero() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        _ = data.into_iter().vec_chunks(0);
    }
}
