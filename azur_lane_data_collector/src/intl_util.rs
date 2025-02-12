use std::mem::take;

use small_fixed_array::{FixedArray, TruncatingInto};

pub trait FixedArrayExt<T> {
    fn push(&mut self, item: T);
}

impl<T> FixedArrayExt<T> for FixedArray<T> {
    fn push(&mut self, item: T) {
        let mut vec = take(self).into_vec();
        vec.reserve_exact(1);
        vec.push(item);
        *self = vec.trunc_into();
    }
}

pub trait IterExt: Iterator + Sized {
    fn collect_fixed_array(self) -> FixedArray<Self::Item> {
        self.collect::<Vec<Self::Item>>().trunc_into()
    }
}

impl<I: Iterator> IterExt for I {}
