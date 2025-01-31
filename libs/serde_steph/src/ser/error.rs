use std::{fmt, io};

use serde::ser;

/// Potential errors to encounter when serializing binary data.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The error originated from the [`io::Write`] implementation.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// A sequence or map tried to serialize itself without a length hint.
    #[error("sequences and maps must provide a length hint")]
    LengthRequired,
    /// Another reason provided by the serializing object.
    #[error("{0}")]
    Custom(String),
}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::Custom(msg.to_string())
    }
}
