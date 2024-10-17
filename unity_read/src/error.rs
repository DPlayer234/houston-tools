//! Error handling type.

/// Error when reading from a Unity FS file or related data structures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The data is invalid.
    #[error("invalid data: {0}")]
    InvalidData(&'static str),

    /// There is a mismatch in expected and received data types.
    #[error("mismatch: expected {expected}, but received {received}")]
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
    #[error("{0}")]
    Unsupported(String),

    /// An I/O error occurred.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// An error during [`binrw`] reading occurred.
    #[error(transparent)]
    BinRw(#[from] binrw::Error),

    /// An error decompressing LZMA-compressed data occurred.
    #[error(transparent)]
    Lzma(#[from] lzma_rs::error::Error),

    /// String data contained invalid UTF-8.
    #[error(transparent)]
    FromUtf8(#[from] std::string::FromUtf8Error),

    /// A different custom error happened.
    //
    // Using anyhow here is kinda overkill and it could just be `Box<dyn Error...>`
    // but I think anyhow provides a better API. And I don't want to deal feature gates.
    #[error(transparent)]
    Custom(anyhow::Error),
}

impl Error {
    /// Constructs a [`Error::Custom`] variant with the provided error.
    pub fn custom<E: Into<anyhow::Error>>(err: E) -> Self {
        Self::Custom(err.into())
    }
}
