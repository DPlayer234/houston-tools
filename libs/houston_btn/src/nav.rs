use std::fmt;
use std::mem::MaybeUninit;

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

impl<'v> Nav<'v> {
    /// Converts this instance to a component custom ID.
    ///
    /// # Notes
    ///
    /// If serialization fails for any reason, this logs the error to the
    /// [registered logger](log). It is assumed that this should _rarely_
    /// happen and simplifies usage of related methods like
    /// [`ButtonValue::to_custom_id`].
    #[must_use]
    pub fn to_custom_id(&self) -> String {
        encoding::encode_custom_id(self.to_slice(&mut MaybeUninit::uninit()))
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

    /// Provides a byte slice representation.
    ///
    /// If already holding a slice, returns that. Otherwise, initializes the
    /// buffer into the provided memory, writes the value, and returns a slice
    /// to the buffer.
    fn to_slice<'a>(&'a self, buf: &'a mut MaybeUninit<encoding::StackBuf>) -> &'a [u8] {
        match self.0 {
            NavInner::Slice(slice) => slice,
            NavInner::Value(data) => {
                // note: no need to drop the buffer
                let buf = buf.write(encoding::StackBuf::new());
                data.write_inner_data(buf);
                buf
            },
        }
    }
}

impl Serialize for Nav<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(self.to_slice(&mut MaybeUninit::uninit()))
    }
}

impl<'v, 'de: 'v> Deserialize<'de> for Nav<'v> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
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

        deserializer.deserialize_bytes(Visitor)
    }
}

struct Hex<'a>(&'a [u8]);

impl fmt::Debug for Hex<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, &b) in self.0.iter().enumerate() {
            if i & 3 == 0 && i != 0 {
                f.write_str("-")?;
            }
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Nav<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            NavInner::Slice(slice) => f.debug_tuple("Slice").field(&Hex(slice)).finish(),
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
