use std::{fmt, io};

use serde::{de, ser};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("sequences and maps must provide a length hint")]
    LengthRequired,
    #[error("invalid utf-8 in data for string")]
    InvalidUtf8,
    #[error("invalid char code")]
    InvalidChar,
    #[error("invalid bool value")]
    InvalidBool,
    #[error("eof reached when more data was expected")]
    UnexpectedEof,
    #[error("types deserializing via any are unsupported")]
    AnyUnsupported,
    #[error("LEB encoded integer overflows target type")]
    IntegerOverflow,
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

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::Custom(msg.to_string())
    }
}
