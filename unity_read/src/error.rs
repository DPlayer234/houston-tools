//! Error handling type.

use std::fmt::{Debug, Display};
use std::error::Error as StdError;

/// Error when reading from a Unity FS file or related data structures.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The data is invalid.
    InvalidData(&'static str),

    /// There is a mismatch in expected and received data types.
    Mismatch {
        // Ideally, this wouldn't hold `String` but `&str`, however not all strings
        // here would be 'static so this would require introducing a lifetime on
        // the error and result types which would make it impossible to cast it to
        // an `anyhow::Error`.

        /// The name of the expected data type.
        expected: String,
        /// The name of the received data type.
        received: String,
    },

    /// The data is unsupported by this library.
    Unsupported(String),

    /// An I/O error occurred.
    Io(std::io::Error),
    /// An error during [`binrw`] reading occurred.
    BinRw(binrw::Error),
    /// An error decompressing LZMA-compressed data occurred.
    Lzma(lzma_rs::error::Error),
    /// String data contained invalid UTF-8.
    FromUtf8(std::string::FromUtf8Error),

    /// A different custom error happened.
    //
    // Using anyhow here is kinda overkill and it could just be `Box<dyn Error...>`
    // but I think anyhow provides a better API. And I don't want to deal feature gates.
    Custom(anyhow::Error),
}

impl Error {
    /// Constructs a [`Error::Custom`] variant with the provided error.
    pub fn custom<E: Into<anyhow::Error>>(err: E) -> Self {
        Self::Custom(err.into())
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Self::FromUtf8(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<binrw::Error> for Error {
    fn from(value: binrw::Error) -> Self {
        Self::BinRw(value)
    }
}

impl From<lzma_rs::error::Error> for Error {
    fn from(value: lzma_rs::error::Error) -> Self {
        Self::Lzma(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidData(msg) => write!(f, "invalid data: {msg}"),
            Self::Mismatch { expected, received } => write!(f, "mismatch: expected {expected}, but received {received}"),
            Self::Unsupported(msg) => f.write_str(msg),

            Self::Io(error) => Display::fmt(error, f),
            Self::BinRw(error) => Display::fmt(error, f),
            Self::Lzma(error) => Display::fmt(error, f),
            Self::FromUtf8(error) => Display::fmt(error, f),
            Self::Custom(error) => Display::fmt(error, f),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::InvalidData(_) |
            Self::Mismatch { .. } |
            Self::Unsupported(_) => None,

            Self::Io(error) => Some(error),
            Self::BinRw(error) => Some(error),
            Self::Lzma(error) => Some(error),
            Self::FromUtf8(error) => Some(error),
            Self::Custom(error) => Some(error.as_ref()),
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn custom_error_source() {
        use super::*;

        #[derive(Debug)]
        struct Custom;
        impl StdError for Custom {}
        impl Display for Custom {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("custom error")
            }
        }

        let err = Error::custom(Custom);
        let source = err.source();
        assert!(source.is_some_and(|e| e.is::<Custom>()), "expected Custom, was: {:?}", source);
    }
}
