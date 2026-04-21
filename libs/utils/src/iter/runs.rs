use std::iter::Peekable;
use std::num::NonZero;

/// Iterator for [`runs`](super::IteratorExt::runs).
pub struct Runs<I: Iterator>(RunsInner<I, ByPartialEq>);

impl<I: Iterator> Runs<I> {
    pub(crate) fn new(iter: I) -> Self {
        Self(RunsInner::new(iter, ByPartialEq))
    }
}

impl<I> Iterator for Runs<I>
where
    I: Iterator,
    I::Item: PartialEq,
{
    type Item = (I::Item, NonZero<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// Iterator for [`runs_by`](super::IteratorExt::runs_by).
pub struct RunsBy<I: Iterator, F>(RunsInner<I, F>);

impl<I: Iterator, F> RunsBy<I, F> {
    pub(crate) fn new(iter: I, pred: F) -> Self {
        Self(RunsInner::new(iter, pred))
    }
}

impl<I, F> Iterator for RunsBy<I, F>
where
    I: Iterator,
    F: Fn(&I::Item, &I::Item) -> bool,
{
    type Item = (I::Item, NonZero<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

struct RunsInner<I: Iterator, P> {
    iter: Peekable<I>,
    pred: P,
}

impl<I, F> RunsInner<I, F>
where
    I: Iterator,
{
    pub(crate) fn new(iter: I, pred: F) -> Self {
        Self {
            iter: iter.peekable(),
            pred,
        }
    }
}

trait Predicate<T: ?Sized> {
    fn eq_by(&self, a: &T, b: &T) -> bool;
}

impl<T, F: Fn(&T, &T) -> bool> Predicate<T> for F {
    fn eq_by(&self, a: &T, b: &T) -> bool {
        (self)(a, b)
    }
}

#[derive(Debug, Clone, Copy)]
struct ByPartialEq;

impl<T: ?Sized + PartialEq> Predicate<T> for ByPartialEq {
    fn eq_by(&self, a: &T, b: &T) -> bool {
        *a == *b
    }
}

impl<I, F> Iterator for RunsInner<I, F>
where
    I: Iterator,
    F: Predicate<I::Item>,
{
    type Item = (I::Item, NonZero<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iter.next()?;
        let mut count = <NonZero<usize>>::MIN;
        while self.iter.next_if(|n| self.pred.eq_by(n, &item)).is_some() {
            count = count.checked_add(1)?;
        }
        Some((item, count))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min, max) = self.iter.size_hint();
        (min.min(1), max)
    }
}
