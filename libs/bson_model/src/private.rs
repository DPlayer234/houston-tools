//! Internal details for use by the proc-macro expansion.

use serde::ser::SerializeSeq as _;
use serde::{Serialize, Serializer};
pub use {bson, serde};

use crate::Filter;

/// Think of this as `Fn<S: Serializer>(&T, S) -> Result<S::Ok, S::Error>`
///
/// Used to support `#[serde(with = "...")]` for filters.
pub trait SerdeWith<T> {
    fn serialize<S: Serializer>(&self, value: &T, serializer: S) -> Result<S::Ok, S::Error>;
}

impl<T, W: SerdeWith<T>> SerdeWith<T> for &W {
    fn serialize<S: Serializer>(&self, value: &T, serializer: S) -> Result<S::Ok, S::Error> {
        (**self).serialize(value, serializer)
    }
}

impl<T: Serialize> SerdeWith<T> for () {
    fn serialize<S: Serializer>(&self, value: &T, serializer: S) -> Result<S::Ok, S::Error> {
        value.serialize(serializer)
    }
}

/// Serializes a value or slice with a given function.
struct WithFn<'a, T: ?Sized, F> {
    value: &'a T,
    with: F,
}

impl<'a, T: ?Sized, F> WithFn<'a, T, F> {
    fn new(value: &'a T, with: F) -> Self {
        Self { value, with }
    }
}

impl<T, F> Serialize for WithFn<'_, T, F>
where
    F: SerdeWith<T>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.with.serialize(self.value, serializer)
    }
}

impl<T, F> Serialize for WithFn<'_, [T], F>
where
    F: SerdeWith<T>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_seq(Some(self.value.len()))?;
        for item in self.value {
            s.serialize_element(&WithFn::new(item, &self.with))?;
        }
        s.end()
    }
}

pub fn serialize_filter_with<T, S, F>(
    value: &Filter<T>,
    serializer: S,
    with: F,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    F: SerdeWith<T>,
{
    macro_rules! ser_var {
        ($index:expr, $name:expr, $value:expr) => {
            serializer.serialize_newtype_variant(
                "Filter",
                $index,
                $name,
                &WithFn::new($value, with),
            )
        };
    }

    match value {
        Filter::Eq(value) => WithFn::new(value, with).serialize(serializer),
        Filter::Ne(value) => ser_var!(0, "$ne", value),
        Filter::Gt(value) => ser_var!(1, "$gt", value),
        Filter::Gte(value) => ser_var!(2, "$gte", value),
        Filter::Lt(value) => ser_var!(3, "$lt", value),
        Filter::Lte(value) => ser_var!(4, "$lte", value),
        Filter::In(list) => ser_var!(5, "$in", list.as_slice()),
        Filter::NotIn(list) => ser_var!(6, "$nin", list.as_slice()),
        Filter::Exists(exists) => {
            serializer.serialize_newtype_variant("Filter", 7, "$exists", exists)
        },
    }
}
