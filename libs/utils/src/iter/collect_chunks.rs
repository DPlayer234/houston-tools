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

    fn tail_len(&self) -> NonZero<usize>
    where
        I: ExactSizeIterator,
    {
        // figure out how many elements are in the tail
        // if len is wrong, this will just be buggy
        NonZero::new(self.inner.len() % self.chunk_size).unwrap_or(self.chunk_size)
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
            Some(drain(iter.take(self.chunk_size.get())))
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if n != 0 {
            self.inner.nth(n * self.chunk_size.get() - 1);
        }
        self.next()
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
        let tail_len = self.tail_len();
        let mut iter = self.inner.by_ref().rev().peekable();
        if iter.peek().is_some() {
            // this figures out the correct capacity for the vec.
            // take never consumes more than `tail` elements.
            let mut chunk: C = drain(iter.take(tail_len.get()));
            // reverse the chunk so the input order is retained
            // note: `rev` on `Take` drains the rest of the iterator so don't
            chunk.as_mut().reverse();
            Some(chunk)
        } else {
            None
        }
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        if n != 0 {
            let tail_len = self.tail_len();
            self.inner
                .nth_back(tail_len.get() + (n - 1) * self.chunk_size.get() - 1);
        }
        self.next_back()
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

/// Helper to collect into a collection and then drain the iterator.
#[inline]
fn drain<I, C>(mut iter: I) -> C
where
    I: Iterator,
    C: FromIterator<I::Item>,
{
    let chunk = iter.by_ref().collect();
    // especially for `Take<I>` this optimizes better than a manual for-loop
    iter.for_each(|_| {});
    chunk
}

#[cfg(test)]
mod tests {
    use std::convert::identity;

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

    #[test]
    #[expect(clippy::iter_nth_zero)]
    fn vec_chunks_nth() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let chunks = || data.iter().copied().collect_chunks::<Vec<_>>(3);

        assert_eq!(chunks().nth(0), Some(vec![1, 2, 3]));
        assert_eq!(chunks().nth(1), Some(vec![4, 5, 6]));
        assert_eq!(chunks().nth(2), Some(vec![7, 8, 9]));
        assert_eq!(chunks().nth(3), Some(vec![10]));
        assert_eq!(chunks().nth(4), None);
    }

    #[test]
    fn vec_chunks_nth_back() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let chunks = || data.iter().copied().collect_chunks::<Vec<_>>(3);

        assert_eq!(chunks().nth_back(0), Some(vec![10]));
        assert_eq!(chunks().nth_back(1), Some(vec![7, 8, 9]));
        assert_eq!(chunks().nth_back(2), Some(vec![4, 5, 6]));
        assert_eq!(chunks().nth_back(3), Some(vec![1, 2, 3]));
        assert_eq!(chunks().nth_back(4), None);
    }

    #[test]
    fn vec_chunks_nth_back_exact() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let chunks = || data.iter().copied().collect_chunks::<Vec<_>>(3);

        assert_eq!(chunks().nth_back(0), Some(vec![7, 8, 9]));
        assert_eq!(chunks().nth_back(1), Some(vec![4, 5, 6]));
        assert_eq!(chunks().nth_back(2), Some(vec![1, 2, 3]));
        assert_eq!(chunks().nth_back(3), None);
    }

    #[test]
    fn option_vec_chunks_some() {
        let data = (1..=10).map(Some).collect::<Vec<_>>();
        let mut chunks = data.into_iter().collect_chunks::<Option<Vec<_>>>(3);

        assert_eq!(chunks.next(), Some(Some(vec![1, 2, 3])));
        assert_eq!(chunks.next(), Some(Some(vec![4, 5, 6])));
        assert_eq!(chunks.next(), Some(Some(vec![7, 8, 9])));
        assert_eq!(chunks.next(), Some(Some(vec![10])));
        assert_eq!(chunks.next(), None);
    }

    #[test]
    fn option_vec_chunks_none() {
        let data = vec![
            Some(1),
            Some(2),
            None,
            Some(4),
            Some(5),
            Some(6),
            Some(7),
            None,
            Some(9),
            Some(10),
        ];
        let mut chunks = data.into_iter().collect_chunks::<Option<Vec<_>>>(3);

        assert_eq!(chunks.next(), Some(None));
        assert_eq!(chunks.next(), Some(Some(vec![4, 5, 6])));
        assert_eq!(chunks.next(), Some(None));
        assert_eq!(chunks.next(), Some(Some(vec![10])));
        assert_eq!(chunks.next(), None);
    }

    #[derive(Debug, PartialEq)]
    struct Interrupt<C>(C);

    impl<A, C: FromIterator<A>> FromIterator<Option<A>> for Interrupt<C> {
        fn from_iter<T: IntoIterator<Item = Option<A>>>(iter: T) -> Self {
            Self(iter.into_iter().map_while(identity).collect())
        }
    }

    // a more intentional example abusing the behavior
    #[test]
    fn interrupt_vec_chunks_none() {
        let data = vec![
            None,
            None,
            None,
            Some(1),
            Some(2),
            None,
            Some(4),
            Some(5),
            Some(6),
            Some(7),
            None,
            Some(9),
            Some(10),
        ];
        let mut chunks = data.into_iter().collect_chunks::<Interrupt<Vec<_>>>(3);

        assert_eq!(chunks.next(), Some(Interrupt(vec![])));
        assert_eq!(chunks.next(), Some(Interrupt(vec![1, 2])));
        assert_eq!(chunks.next(), Some(Interrupt(vec![4, 5, 6])));
        assert_eq!(chunks.next(), Some(Interrupt(vec![7])));
        assert_eq!(chunks.next(), Some(Interrupt(vec![10])));
        assert_eq!(chunks.next(), None);
    }
}
