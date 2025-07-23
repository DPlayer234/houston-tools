use std::mem::take;

use small_fixed_array::{FixedArray, TruncatingInto};

pub trait FixedArrayExt<T> {
    /// Absolutely "efficient" way to add an item.
    ///
    /// This will effectively end up copying the entire array to new storage.
    fn push(&mut self, item: T);

    fn extend_from_array(&mut self, other: Self);
}

impl<T> FixedArrayExt<T> for FixedArray<T> {
    fn push(&mut self, item: T) {
        let mut vec = take(self).into_vec();
        vec.reserve_exact(1);
        vec.push(item);
        *self = vec.trunc_into();
    }

    fn extend_from_array(&mut self, other: Self) {
        let mut vec = take(self).into_vec();
        vec.extend(other);
        *self = vec.trunc_into();
    }
}

pub trait IterExt: Iterator + Sized {
    fn collect_fixed_array(self) -> FixedArray<Self::Item> {
        self.collect::<Vec<Self::Item>>().trunc_into()
    }
}

pub trait TryIterExt<T, E>: Iterator<Item = Result<T, E>> + Sized {
    fn try_collect<C: FromIterator<T>>(self) -> Result<C, E> {
        self.collect()
    }

    fn try_collect_fixed_array(self) -> Result<FixedArray<T>, E> {
        self.try_collect::<Vec<_>>().map(TruncatingInto::trunc_into)
    }
}

impl<I: Iterator> IterExt for I {}
impl<I: Iterator<Item = Result<T, E>>, T, E> TryIterExt<T, E> for I {}
