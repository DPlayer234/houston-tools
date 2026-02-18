//! Error handling types.
//!
//! The serde docs suggest a data format should expose one shared error type.
//! So... we do. And also a result type.

use std::{fmt, io};

use serde_core::{de, ser};

#[expect(missing_docs)]
pub type Result<T> = std::result::Result<T, Error>;

/// Potential errors to encounter when serializing or deserializing binary data.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Another reason provided by the object implementation.
    #[error("{0}")]
    Custom(String),
    /// The error originated from the [`io::Write`] or [`io::Read`]
    /// implementation.
    #[error(transparent)]
    Io(#[from] io::Error),

    /// A struct, sequence, or map tried to serialize itself without a length.
    ///
    /// Calling [`collect_seq`](ser::Serializer::collect_seq) or
    /// [`collect_map`](ser::Serializer::collect_map) with an iterator that
    /// doesn't provide an exact size hint will also lead to this error.
    #[error("structs, sequences, and maps must specify a length")]
    LengthRequired,
    /// A struct, sequence, or map tried to serialize itself with the wrong
    /// length.
    ///
    /// Calling [`collect_seq`](ser::Serializer::collect_seq) or
    /// [`collect_map`](ser::Serializer::collect_map) with an iterator that
    /// provides an exact-yet-incorrect size hint will also lead to this error.
    #[error("length for struct, sequence, or map was incorrect")]
    LengthIncorrect,

    /// Tried to deserialize a [`str`] value but the corresponding bytes did not
    /// contain fully valid UTF-8.
    #[error("invalid utf-8 in bytes for string")]
    InvalidUtf8,
    /// Tried to deserialize a [`char`] value but the character code was
    /// invalid.
    #[error("invalid char code")]
    InvalidChar,
    /// Tried to deserialize a [`bool`] value but it wasn't 0 or 1.
    #[error("invalid bool value")]
    InvalidBool,
    /// Tried to deserialize an [`Option`] with an invalid discriminator.
    #[error("invalid option discriminator")]
    InvalidOption,
    /// Tried to deserialize a struct or sequence but less elements were read
    /// from the deserializer than specified by the length prefix.
    #[error("read less seq elements than specified by length prefix")]
    ShortSeqRead,
    /// [`deserialize_any`](de::Deserializer::deserialize_any) or
    /// [`deserialize_ignored_any`](de::Deserializer::deserialize_ignored_any)
    /// were called.
    #[error("deserializing any is unsupported")]
    AnyUnsupported,
    /// While deserializing LEB128 integer data, the data overflowed the target
    /// type.
    #[error("LEB128 encoded integer overflows target type")]
    IntegerOverflow,
    /// Past the expected end of the deserialized object were trailing bytes.
    #[error("trailing bytes past the end of the deserialized value")]
    TrailingBytes,
}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::Custom(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::Custom(msg.to_string())
    }
}
