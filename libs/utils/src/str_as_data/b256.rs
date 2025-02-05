//! Encodes bytes as "base 65535".
//!
//! Encodes each byte as one [`char`] of the output with the equivalent code
//! point value. A start and end marker are added.
//!
//! The exact format is as follows:
//!
//! - The prefix is added: It is always `#`.
//! - Each byte is converted to a [`char`] with the code point equal to the
//!   byte's numeric value.
//! - The suffix is added: It is always `&`.
//!
//! Decoding applies these rules in reverse, with only [`char`] codes in the
//! range `0x0` to `0xFF` being allowed.

use std::{fmt, io};

use super::Error;

/// The maximum byte length a specified count of characters may decode to.
///
/// This can be used to reserve space in a buffer.
pub const fn max_byte_len(char_count: usize) -> usize {
    char_count - 2
}

/// Encodes bytes as "base 256", returning a [`String`] with the result.
///
/// This is equivalent to using [`encode`] with a [`String`].
///
/// Use [`from_str`] to reverse the operation.
#[must_use]
pub fn to_string(bytes: &[u8]) -> String {
    let expected_size = 2 + bytes.len() + (bytes.len() >> 1);
    let mut result = String::with_capacity(expected_size);

    encode(&mut result, bytes).expect("write to String cannot fail");

    result
}

/// Encodes bytes as "base 65535", writing them to a buffer.
///
/// Use [`decode`] to reverse the operation.
///
/// This can only return an [`Err`] if the `writer` does so.
pub fn encode<W: fmt::Write>(mut writer: W, bytes: &[u8]) -> fmt::Result {
    writer.write_char('#')?;
    for b in bytes {
        writer.write_char(char::from(*b))?;
    }

    writer.write_char('&')
}

/// Equivalent to [`decode`] with a [`Vec<u8>`] as the buffer.
pub fn from_str(input: &str) -> Result<Vec<u8>, Error> {
    let expected_size = input.len();
    let mut result = Vec::with_capacity(expected_size);

    decode(&mut result, input)?;
    Ok(result)
}

/// Decodes a string holding "base 65536" data, writing the bytes to a buffer.
///
/// Returns [`Err`] if the data is invalid, lacks the required markers, or the
/// writer returned an error.
pub fn decode<W: io::Write>(mut writer: W, input: &str) -> Result<(), Error> {
    let input = strip_input(input)?;

    for c in input.chars() {
        let byte = u8::try_from(c).map_err(|_| Error::ContentFormat)?;
        writer.write_all(&[byte])?;
    }

    Ok(())
}

fn strip_input(input: &str) -> Result<&str, Error> {
    input
        // strip the start marker
        .strip_prefix('#')
        // strip the end marker
        .and_then(|s| s.strip_suffix('&'))
        .ok_or(Error::PrefixSuffix)
}
