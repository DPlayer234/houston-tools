//! Internal details for use by the proc-macro expansion.

use serde::{Serialize, Serializer};
pub use {bson, serde};

use crate::Filter;

pub trait SerdeWith<T> {
    fn serialize<S: Serializer>(&self, value: &T, serializer: S) -> Result<S::Ok, S::Error>;
}

pub fn wrap_filter_with<T, S, F>(
    value: &Filter<T>,
    serializer: S,
    with: F,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    F: SerdeWith<T>,
{
    struct With<'a, T, F> {
        value: &'a T,
        with: &'a F,
    }

    impl<T, F> Serialize for With<'_, T, F>
    where
        F: SerdeWith<T>,
    {
        fn serialize<S>(&self, __s: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            SerdeWith::serialize(self.with, self.value, __s)
        }
    }

    let with = &with;
    value
        .as_ref()
        .map(|value| With { value, with })
        .serialize(serializer)
}
