use std::fmt;
use std::iter::{Fuse, FusedIterator};
use std::marker::PhantomData;
use std::num::NonZero;

/// Iterates over chunks of values, each yielded as a collection.
pub struct CollectChunks<I, C> {
    // needs to be fused to avoid calling `next` and it returning `Some` again. we don't want to
    // support cases where `None` "cancels" a chunk and further iterator just continues chunks from
    // there.
    inner: Fuse<I>,
    chunk_size: NonZero<usize>,
    chunk_marker: PhantomData<C>,
}

impl<I: Iterator, C> CollectChunks<I, C> {
    pub(super) fn new(iter: I, chunk_size: usize) -> Self {
        let chunk_size = NonZero::new(chunk_size).expect("chunk_size must not be zero");
        Self {
            inner: iter.fuse(),
            chunk_size,
            chunk_marker: PhantomData,
        }
    }
}

impl<I: Clone, C> Clone for CollectChunks<I, C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            ..*self
        }
    }
}

impl<I: fmt::Debug, C> fmt::Debug for CollectChunks<I, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CollectChunks")
            .field("chunk_size", &self.chunk_size)
            .field("inner", &self.inner)
            .field("collection_ty", &self.chunk_marker)
            .finish()
    }
}

impl<I, C> Iterator for CollectChunks<I, C>
where
    I: Iterator,
    C: FromIterator<I::Item>,
{
    type Item = C;

    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.inner.by_ref().peekable();
        if iter.peek().is_some() {
            // this figures out the correct capacity for the collection if the iterator
            // provides a useful size hint. take never consumes more than size elements.
            Some(iter.take(self.chunk_size.get()).collect())
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.inner.size_hint();
        (
            lower.div_ceil(self.chunk_size.get()),
            upper.map(|upper| upper.div_ceil(self.chunk_size.get())),
        )
    }
}

impl<I, C> DoubleEndedIterator for CollectChunks<I, C>
where
    I: DoubleEndedIterator + ExactSizeIterator,
    // the `AsMut` bound is needed so we can reverse the buffer after collecting
    // otherwise we'd need an intermediate buffer of a controlled type
    C: FromIterator<I::Item> + AsMut<[I::Item]>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        // figure out how many elements are in the tail
        // if len is wrong, this will just be buggy
        let tail = NonZero::new(self.inner.len() % self.chunk_size).unwrap_or(self.chunk_size);

        let mut iter = self.inner.by_ref().rev().peekable();
        if iter.peek().is_some() {
            // this figures out the correct capacity for the vec.
            // take never consumes more than `tail` elements.
            let mut chunk: C = iter.take(tail.get()).collect();
            // reverse the chunk so the input order is retained
            // note: `rev` on `Take` drains the rest of the iterator so don't
            chunk.as_mut().reverse();
            Some(chunk)
        } else {
            None
        }
    }
}

impl<I, C> ExactSizeIterator for CollectChunks<I, C>
where
    I: ExactSizeIterator,
    C: FromIterator<I::Item>,
{
}

// unconditionally fused because it's backed by `Fuse<I>`
impl<I, C> FusedIterator for CollectChunks<I, C>
where
    I: Iterator,
    C: FromIterator<I::Item>,
{
}

#[cfg(test)]
mod tests {
    use crate::iter::IteratorExt as _;

    #[test]
    fn vec_chunks() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut chunks = data.into_iter().collect_chunks::<Vec<_>>(3);

        assert_eq!(chunks.next(), Some(vec![1, 2, 3]));
        assert_eq!(chunks.next(), Some(vec![4, 5, 6]));
        assert_eq!(chunks.next(), Some(vec![7, 8, 9]));
        assert_eq!(chunks.next(), Some(vec![10]));
        assert_eq!(chunks.next(), None);
    }

    #[test]
    fn vec_chunks_back() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut chunks = data.into_iter().collect_chunks::<Vec<_>>(3);

        assert_eq!(chunks.next_back(), Some(vec![10]));
        assert_eq!(chunks.next_back(), Some(vec![7, 8, 9]));
        assert_eq!(chunks.next_back(), Some(vec![4, 5, 6]));
        assert_eq!(chunks.next_back(), Some(vec![1, 2, 3]));
        assert_eq!(chunks.next_back(), None);
    }

    #[test]
    fn vec_chunks_mixed() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut chunks = data.into_iter().collect_chunks::<Vec<_>>(3);

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
            let chunks = data.into_iter().collect_chunks::<Vec<_>>(chunk_size);
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
        _ = data.into_iter().collect_chunks::<Vec<_>>(0);
    }
}
