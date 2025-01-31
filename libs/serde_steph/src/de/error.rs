use std::{fmt, io};

use serde::de;

/// Potential errors to encounter when deserializing binary data.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The error originated from the [`io::Read`] implementation.
    #[error(transparent)]
    Io(#[from] io::Error),
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
    /// While reading LEB128 integer data, the data overflowed the target type.
    #[error("LEB encoded integer overflows target type")]
    IntegerOverflow,
    /// Another reason provided by the deserializing object.
    #[error("{0}")]
    Custom(String),
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::Custom(msg.to_string())
    }
}
