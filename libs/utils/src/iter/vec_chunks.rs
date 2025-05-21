use std::iter::Take;

/// Iterates over chunks of values, yielded as vectors.
pub struct VecChunks<I> {
    chunk_size: usize,
    inner: I,
}

impl<I> VecChunks<I> {
    pub(super) fn new(iter: I, chunk_size: usize) -> Self {
        assert!(chunk_size != 0, "chunk_size must not be zero");
        Self {
            inner: iter,
            chunk_size,
        }
    }
}

impl<I: Iterator> VecChunks<I> {
    fn next_chunk_iter(&mut self) -> Take<&mut I> {
        self.inner.by_ref().take(self.chunk_size)
    }
}

impl<I: Iterator> Iterator for VecChunks<I> {
    type Item = Vec<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        // this figures out the correct capacity for the vec if the iterator provides
        // a useful size hint. take never consumes more than size elements.
        let chunk: Vec<I::Item> = self.next_chunk_iter().collect();
        (!chunk.is_empty()).then_some(chunk)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.inner.size_hint();
        (
            lower.div_ceil(self.chunk_size),
            upper.map(|upper| upper.div_ceil(self.chunk_size)),
        )
    }
}

// exact-size is reasonable, but double-ended isn't because there is no way to
// always know how large the last chunk is. and exact-size can't be fully
// trusted for that either.
impl<I: ExactSizeIterator> ExactSizeIterator for VecChunks<I> {
    fn len(&self) -> usize {
        self.inner.len().div_ceil(self.chunk_size)
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
    fn vec_chunks_len() {
        fn check(data: Vec<i32>, chunk_size: usize, expected_len: usize) {
            let chunks = data.into_iter().vec_chunks(chunk_size);
            assert_eq!(chunks.len(), expected_len);
        }

        check(vec![], 3, 0);
        check(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 3, 4);
        check(vec![1, 2, 3, 4, 5, 6, 7, 8, 9], 3, 3);
        check(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11], 3, 4);
        check(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], 3, 4);
        check(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13], 3, 5);
    }

    #[test]
    #[should_panic = "chunk_size must not be zero"]
    fn vec_chunks_zero() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        _ = data.into_iter().vec_chunks(0);
    }
}
