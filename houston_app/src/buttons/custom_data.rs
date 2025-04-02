use std::fmt;
use std::marker::PhantomData;

use serde::de::{Deserialize, Deserializer, Error};
use serde::ser::{Serialize, Serializer};

use super::{ButtonArgsRef, encoding};
use crate::prelude::*;

/// Represents custom data for another menu.
#[derive(Debug, Clone)]
pub struct CustomData<'v>(CustomDataInner<'v>);

#[derive(Debug, Clone)]
enum CustomDataInner<'v> {
    #[doc(hidden)]
    Slice(&'v [u8]),
    #[doc(hidden)]
    Args(ButtonArgsRef<'v>),
}

macro_rules! to_slice {
    ($c:expr => $buf:ident) => {
        match $c.0 {
            CustomDataInner::Slice(slice) => slice,
            CustomDataInner::Args(args) => {
                $buf = encoding::StackBuf::new();
                encoding::write_button_args(&mut $buf, args);
                &$buf
            },
        }
    };
}

impl Serialize for CustomData<'_> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut buf;
        let slice = to_slice!(*self => buf);
        serializer.serialize_bytes(slice)
    }
}

impl<'v, 'de: 'v> Deserialize<'de> for CustomData<'v> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor<'de>(PhantomData<CustomData<'de>>);

        impl<'de> serde::de::Visitor<'de> for Visitor<'de> {
            type Value = CustomData<'de>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("custom data bytes")
            }

            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(CustomData::from_slice(v))
            }
        }

        deserializer.deserialize_bytes(Visitor(PhantomData))
    }
}

impl<'v> CustomData<'v> {
    /// Gets an empty value.
    #[cfg(test)]
    pub const EMPTY: Self = Self::from_slice(&[]);

    /// Converts this instance to a component custom ID.
    #[must_use]
    pub fn to_custom_id(&self) -> String {
        let mut buf;
        let slice = to_slice!(*self => buf);
        encoding::encode_custom_id(slice)
    }

    #[must_use]
    const fn from_slice(slice: &'v [u8]) -> Self {
        Self(CustomDataInner::Slice(slice))
    }

    /// Creates an instance from [`ButtonArgs`].
    #[must_use]
    pub(super) fn from_button_args(args: ButtonArgsRef<'v>) -> Self {
        Self(CustomDataInner::Args(args))
    }
}
