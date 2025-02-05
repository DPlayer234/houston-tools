//! Provides ways to encode binary data as valid UTF-8 strings and
//! convert those strings back into binary data.
//!
//! To avoid trimming of white-space at the start and end of strings,
//! every string output has delimiters added.
//!
//! Each format provides a pair of `encode` and `decode` methods. "Encoding"
//! takes bytes and returns strings whereas "decoding" does the inverse.
//!
//! Additionally there are `to_string` and `from_str` convenience methods.
//!
//! See the documentation of sub-modules for more information.

pub mod b20bit;
pub mod b256;
pub mod b65536;
#[cfg(test)]
mod tests;

/// Error decoding [`str_as_data`](self) data.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The data was invalid.
    #[error("input data is invalid")]
    #[deprecated = "not returned anymore, to be removed"]
    Invalid,
    /// The written buffer returned an error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("length invalid for metadata")]
    LenMismatch,
    #[error("prefix or suffix invalid for data format")]
    PrefixSuffix,
    #[error("content char code out of range for format")]
    ContentFormat,
}
