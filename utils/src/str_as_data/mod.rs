//! Provides ways to encode binary data as valid UTF-8 strings and
//! convert those strings back into binary data.
//!
//! To avoid trimming of white-space at the start and end of strings,
//! every string output has delimiters added.
//!
//! The `to_*` and `from_*` methods output [`String`] and [`Vec<u8>`],
//! while `encode_*` and `decode_*` write to buffers.
//! The input cannot be a reader.
//!
//! The current supported formats are:
//!
//! ## Base 256
//!
//! Via [`to_b256`]/[`encode_b256`] and [`from_b256`]/[`decode_b256`]:
//! Encodes each byte as one [`char`] of the output with the equivalent code
//! point value.
//!
//! ## Base 65536:
//!
//! Via [`to_b65536`]/[`encode_b65536`] and [`from_b65536`]/[`decode_b65536`]:
//! Encodes pairs of bytes as one [`char`] of the output with a unique code
//! point for each possible input.

mod b256;
mod b65536;

pub use b256::*;
pub use b65536::*;

/// Error decoding [`str_as_data`](self) data.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The data was invalid.
    #[error("input data is invalid")]
    Invalid,
    /// The written buffer returned an error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod test;
