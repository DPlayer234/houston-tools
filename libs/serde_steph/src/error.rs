//! Error handling types.
//!
//! The serde docs suggest a data format should expose one shared error type.
//! So... we do. And also a result type.

use std::{fmt, io};

use serde::{de, ser};

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

    /// A sequence or map tried to serialize itself without a length hint.
    #[error("sequences and maps must provide a length hint")]
    LengthRequired,

    /// Tries to deserialize a [`str`] value but it contained invalid UTF-8.
    #[error("invalid utf-8 in data for string")]
    InvalidUtf8,
    /// Tried to deserialize a [`char`] value but its code was invalid.
    #[error("invalid char code")]
    InvalidChar,
    /// Tried to deserialize a [`bool`] value but it wasn't 0 or 1.
    #[error("invalid bool value")]
    InvalidBool,
    /// Tried to deserialize an [`Option`] with an invalid discriminator.
    #[error("invalid option discriminator")]
    InvalidOption,
    /// A type tried to use [`de::Deserializer::deserialize_any`].
    #[error("types deserializing via any are unsupported")]
    AnyUnsupported,
    /// While deserializing LEB128 integer data, the data overflowed the target
    /// type.
    #[error("LEB encoded integer overflows target type")]
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
