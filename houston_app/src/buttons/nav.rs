use std::fmt;
use std::marker::PhantomData;

use serde::de::{Deserialize, Deserializer, Error};
use serde::ser::{Serialize, Serializer};

use super::{ButtonArgsRef, encoding};
use crate::prelude::*;

/// Represents a reference to another menu.
///
/// This either references the actual view or its serialized form.
#[derive(Debug, Clone)]
pub struct Nav<'v>(NavInner<'v>);

#[derive(Debug, Clone, Copy)]
enum NavInner<'v> {
    Slice(&'v [u8]),
    Args(ButtonArgsRef<'v>),
}

macro_rules! to_slice {
    ($c:expr => $buf:ident) => {
        match $c.0 {
            NavInner::Slice(slice) => slice,
            NavInner::Args(args) => {
                $buf = encoding::StackBuf::new();
                encoding::write_button_args(&mut $buf, args);
                &$buf
            },
        }
    };
}

impl Serialize for Nav<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut buf;
        let slice = to_slice!(*self => buf);
        serializer.serialize_bytes(slice)
    }
}

impl<'v, 'de: 'v> Deserialize<'de> for Nav<'v> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor<'de>(PhantomData<Nav<'de>>);

        impl<'de> serde::de::Visitor<'de> for Visitor<'de> {
            type Value = Nav<'de>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("custom data bytes")
            }

            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Nav::from_slice(v))
            }
        }

        deserializer.deserialize_bytes(Visitor(PhantomData))
    }
}

impl<'v> Nav<'v> {
    /// Converts this instance to a component custom ID.
    #[must_use]
    pub fn to_custom_id(&self) -> String {
        let mut buf;
        let slice = to_slice!(*self => buf);
        encoding::encode_custom_id(slice)
    }

    #[must_use]
    pub(super) const fn from_slice(slice: &'v [u8]) -> Self {
        Self(NavInner::Slice(slice))
    }

    /// Creates an instance from [`ButtonArgs`].
    #[must_use]
    pub(super) fn from_button_args(args: ButtonArgsRef<'v>) -> Self {
        Self(NavInner::Args(args))
    }
}
