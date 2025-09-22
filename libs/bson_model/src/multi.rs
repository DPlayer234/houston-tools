use std::ops::{Deref, DerefMut};

use serde::Serialize;
use serde_with::SerializeAs;
use serde_with::ser::SerializeAsWrap;
use small_fixed_array::{FixedArray, ValidLength as _};

/// A semi-opaque wrapper around a boxed slice.
///
/// Derefs to `[T]` and can be iterated.
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct Multi<T>(FixedArray<T>);

impl<T> Multi<T> {
    /// Returns the number of elements in the slice.
    pub fn len(&self) -> usize {
        self.0.len().to_usize()
    }

    /// Returns `true` if the slice has a length of 0.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T> Deref for Multi<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl<T> DerefMut for Multi<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_slice_mut()
    }
}

impl<A> FromIterator<A> for Multi<A> {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        let boxed = iter.into_iter().collect::<Box<[A]>>();
        match FixedArray::try_from(boxed) {
            Ok(fixed) => Self(fixed),
            #[cfg(target_pointer_width = "16")]
            Err(_) => panic!("Multi len should not overflow u16"),
            #[cfg(not(target_pointer_width = "16"))]
            Err(_) => panic!("Multi len should not overflow u32"),
        }
    }
}

impl<T> IntoIterator for Multi<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_vec().into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Multi<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Multi<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<T, U> SerializeAs<Multi<T>> for Multi<U>
where
    U: SerializeAs<T>,
{
    fn serialize_as<S>(source: &Multi<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(source.iter().map(|item| SerializeAsWrap::<T, U>::new(item)))
    }
}
