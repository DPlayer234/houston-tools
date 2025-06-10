/// Iterates over chunks of values, yielded as vectors.
#[derive(Debug, Clone)]
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

impl<I: Iterator> Iterator for VecChunks<I> {
    type Item = Vec<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        // this figures out the correct capacity for the vec if the iterator provides
        // a useful size hint. take never consumes more than size elements.
        let chunk: Vec<I::Item> = self.inner.by_ref().take(self.chunk_size).collect();
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

impl<I> DoubleEndedIterator for VecChunks<I>
where
    I: DoubleEndedIterator + ExactSizeIterator,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        // figure out how many elements are in the tail
        // if len is wrong, this will just be buggy
        let tail = match self.inner.len() % self.chunk_size {
            0 => self.chunk_size,
            t => t,
        };

        // this figures out the correct capacity for the vec.
        // take never consumes more than `tail` elements.
        let mut chunk: Vec<I::Item> = self.inner.by_ref().rev().take(tail).collect();
        if !chunk.is_empty() {
            // reverse the chunk so the input order is retained
            // note: `rev` on `Take` drains the rest of the iterator so don't
            chunk.reverse();
            Some(chunk)
        } else {
            None
        }
    }
}

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
    fn vec_chunks_back() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut chunks = data.into_iter().vec_chunks(3);

        assert_eq!(chunks.next_back(), Some(vec![10]));
        assert_eq!(chunks.next_back(), Some(vec![7, 8, 9]));
        assert_eq!(chunks.next_back(), Some(vec![4, 5, 6]));
        assert_eq!(chunks.next_back(), Some(vec![1, 2, 3]));
        assert_eq!(chunks.next_back(), None);
    }

    #[test]
    fn vec_chunks_mixed() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut chunks = data.into_iter().vec_chunks(3);

        assert_eq!(chunks.next(), Some(vec![1, 2, 3]));
        assert_eq!(chunks.next_back(), Some(vec![10]));
        assert_eq!(chunks.next(), Some(vec![4, 5, 6]));
        assert_eq!(chunks.next_back(), Some(vec![7, 8, 9]));
        assert_eq!(chunks.next(), None);
        assert_eq!(chunks.next_back(), None);
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
