use std::fmt;
use std::marker::PhantomData;

use serde::de::{Deserialize, Deserializer, Error};
use serde::ser::{Serialize, Serializer};

use super::{ButtonValue, encoding};

/// Represents a reference to another menu.
///
/// This either references the actual view or its serialized form.
#[derive(Clone)]
pub struct Nav<'v>(NavInner<'v>);

#[derive(Clone, Copy)]
enum NavInner<'v> {
    Slice(&'v [u8]),
    Value(&'v dyn SerializeCustomIdToStackBuf),
}

macro_rules! to_slice {
    ($c:expr => $buf:ident) => {
        match $c.0 {
            NavInner::Slice(slice) => slice,
            NavInner::Value(data) => {
                $buf = encoding::StackBuf::new();
                data.write_inner_data(&mut $buf);
                &$buf
            },
        }
    };
}

impl<'v> Nav<'v> {
    /// Converts this instance to a component custom ID.
    #[must_use]
    pub fn to_custom_id(&self) -> String {
        let mut buf;
        let slice = to_slice!(*self => buf);
        encoding::encode_custom_id(slice)
    }

    /// Creates a [`Nav`] from a byte slice of serialized data.
    #[must_use]
    pub const fn from_slice(slice: &'v [u8]) -> Self {
        Self(NavInner::Slice(slice))
    }

    /// Creates a [`Nav`] from a [`ButtonValue`].
    #[must_use]
    pub fn from_button_value<T>(args: &'v T) -> Self
    where
        T: ButtonValue + Serialize + fmt::Debug,
    {
        Self(NavInner::Value(args))
    }
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
                formatter.write_str("nav borrowed bytes")
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

impl fmt::Debug for Nav<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            NavInner::Slice(slice) => f.debug_tuple("Slice").field(&slice).finish(),
            NavInner::Value(args) => f.debug_tuple("Value").field(&args).finish(),
        }
    }
}

/// Provides a dyn-compatible wrapper trait for serializing arbitrary structs
/// into the encoding format.
trait SerializeCustomIdToStackBuf: fmt::Debug + Send + Sync {
    fn write_inner_data(&self, buf: &mut encoding::StackBuf);
}

impl<T> SerializeCustomIdToStackBuf for T
where
    T: ButtonValue + Serialize + fmt::Debug,
{
    fn write_inner_data(&self, buf: &mut encoding::StackBuf) {
        encoding::write_inner_data(buf, self);
    }
}
