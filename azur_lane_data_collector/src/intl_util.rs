use std::collections::HashSet;
use std::hash::Hash;
use std::mem::take;

use small_fixed_array::{FixedArray, FixedString, ValidLength};

pub trait IntoFixed<LenT> {
    type Fixed;

    fn into_fixed(self) -> Self::Fixed;
}

impl<LenT: ValidLength> IntoFixed<LenT> for String {
    type Fixed = FixedString<LenT>;

    fn into_fixed(self) -> Self::Fixed {
        self.into_boxed_str()
            .try_into()
            .expect("string len must fit into fixed string")
    }
}

impl<T, LenT: ValidLength> IntoFixed<LenT> for Vec<T> {
    type Fixed = FixedArray<T, LenT>;

    fn into_fixed(self) -> Self::Fixed {
        match self.into_boxed_slice().try_into() {
            Ok(fixed) => fixed,
            Err(_) => panic!("slice len must fit into fixed array"),
        }
    }
}

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
        *self = vec.into_fixed();
    }

    fn extend_from_array(&mut self, other: Self) {
        let mut vec = take(self).into_vec();
        // `<Vec<T>>::extend(Vec<T>)` uses specialization
        vec.extend(other.into_vec());
        *self = vec.into_fixed();
    }
}

pub trait IterExt: Iterator + Sized {
    fn collect_fixed_array(self) -> FixedArray<Self::Item> {
        self.collect::<Vec<Self::Item>>().into_fixed()
    }

    #[expect(clippy::wrong_self_convention)]
    fn is_unique(self) -> bool
    where
        Self::Item: Hash + Eq,
    {
        let mut set = HashSet::new();

        for item in self {
            if !set.insert(item) {
                return false;
            }
        }

        true
    }
}

pub trait TryIterExt: Iterator<Item = Result<Self::Ok, Self::Err>> + Sized {
    type Ok;
    type Err;

    fn try_collect<C: FromIterator<Self::Ok>>(self) -> Result<C, Self::Err> {
        self.collect()
    }

    fn try_collect_fixed_array(self) -> Result<FixedArray<Self::Ok>, Self::Err> {
        self.try_collect().map(Vec::into_fixed)
    }
}

impl<I: Iterator> IterExt for I {}
impl<I: Iterator<Item = Result<T, E>>, T, E> TryIterExt for I {
    type Ok = T;
    type Err = E;
}
