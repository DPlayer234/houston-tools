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

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

/// Iterator for [`runs_by`](super::IteratorExt::runs_by).
pub struct RunsBy<I: Iterator, F>(RunsInner<I, EqFn<F>>);

impl<I: Iterator, F> RunsBy<I, F> {
    pub(crate) fn new(iter: I, eq: F) -> Self {
        Self(RunsInner::new(iter, EqFn(eq)))
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

/// Iterator for [`runs_by_key`](super::IteratorExt::runs_by_key).
pub struct RunsByKey<I: Iterator, F>(RunsInner<I, KeyEqFn<F>>);

impl<I: Iterator, F> RunsByKey<I, F> {
    pub(crate) fn new(iter: I, f: F) -> Self {
        Self(RunsInner::new(iter, KeyEqFn(f)))
    }
}

impl<I, F, K> Iterator for RunsByKey<I, F>
where
    I: Iterator,
    F: Fn(&I::Item) -> &K,
    K: ?Sized + PartialEq,
{
    type Item = (I::Item, NonZero<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
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
    fn eq(&self, a: &T, b: &T) -> bool;
}

#[derive(Debug, Clone, Copy)]
struct ByPartialEq;

impl<T: ?Sized + PartialEq> Predicate<T> for ByPartialEq {
    fn eq(&self, a: &T, b: &T) -> bool {
        *a == *b
    }
}

#[derive(Debug, Clone, Copy)]
struct EqFn<F>(F);

impl<T: ?Sized, F: Fn(&T, &T) -> bool> Predicate<T> for EqFn<F> {
    fn eq(&self, a: &T, b: &T) -> bool {
        (self.0)(a, b)
    }
}

#[derive(Debug, Clone, Copy)]
struct KeyEqFn<F>(F);

impl<T: ?Sized, K: ?Sized + PartialEq, F: Fn(&T) -> &K> Predicate<T> for KeyEqFn<F> {
    fn eq(&self, a: &T, b: &T) -> bool {
        *(self.0)(a) == *(self.0)(b)
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
        while self.iter.next_if(|n| self.pred.eq(n, &item)).is_some() {
            count = count.checked_add(1).expect("run length overflows usize");
        }
        Some((item, count))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min, max) = self.iter.size_hint();
        (min.min(1), max)
    }
}
