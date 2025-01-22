//! Encodes bytes as "base 65535".
//!
//! Bytes will be paired. The combined value of each pair will mapped to UTF-8
//! characters and the sequence is then joined. Both a start and end marker will
//! be added, depending on the byte count.
//!
//! The exact format is as follows:
//!
//! - The prefix is added: It is `&` if the byte count is even, otherwise it is
//!   `%`.
//! - The byte slice is chunked into 16-bit pieces. Each piece is regarded as a
//!   little-endian [`u16`]. These [`u16`] are in turn mapped to [`char`] with
//!   rules described further down. For a byte slice with odd count, the
//!   sequence is treated as if it had an additional null byte at the end.
//! - The suffix is added: It is always `&`.
//!
//! [`u16`] are converted to [`char`] as follows:
//!
//! - If the value is `0x0` to `0xD7FF`: The [`char`] with the code point equal
//!   to the value is used.
//! - If the value is `0xD800` to `0xFFFF`: The [`char`] with the code point
//!   equal to the value plus `0x800` is used.
//!
//! These rules ensure only valid unicode code points are in the output.
//!
//! Decoding applies these rules in reverse, with only [`char`] codes in the
//! range `0x0` to `0xD7FF` and `0xE000` to `0x107FF` being allowed. The prefix
//! is used to determine whether to treat the last char as a single byte
//! or a pair.

use std::{fmt, io};

use super::Error;

/// The maximum byte length a specified count of characters may decode to.
///
/// This can be used to reserve space in a buffer.
pub const fn max_byte_len(char_count: usize) -> usize {
    (char_count - 2) * 2
}

/// Encodes bytes as "base 65535", returning a [`String`] with the result.
///
/// This is equivalent to using [`encode`] with a [`String`].
///
/// Use [`from_str`] to reverse the operation.
#[must_use]
pub fn to_string(bytes: &[u8]) -> String {
    // Testing indicates more than 100% is normal, usually about ~130%.
    // But more is still common and more than 200% is rare, so we go for that.
    let expected_size = 2 + (bytes.len() << 1);
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
    let skip_last = bytes.len() % 2 != 0;
    writer.write_char(match skip_last {
        false => '&',
        true => '%',
    })?;

    let mut iter = bytes.chunks_exact(2);
    for chunk in iter.by_ref() {
        // Conversion cannot fail and check is optimized out.
        let chunk = <[u8; 2]>::try_from(chunk).unwrap();
        writer.write_char(bytes_to_char(chunk))?;
    }

    if let &[last] = iter.remainder() {
        let chunk = [last, 0];
        writer.write_char(bytes_to_char(chunk))?;
    }

    writer.write_char('&')
}

/// Decodes a string holding "base 65536" data.
///
/// Returns [`Err`] if the data is invalid or lacks the required markers.
pub fn from_str(input: &str) -> Result<Vec<u8>, Error> {
    // Extending the logic in `to_string`, less than ~130% is also common.
    // This almost always has enough space and rarely leads to more than
    // half the capacity going entirely unused.
    let expected_size = input.len().saturating_sub(2);
    let mut result = Vec::with_capacity(expected_size);

    decode(&mut result, input)?;
    Ok(result)
}

/// Decodes a string holding "base 65536" data, writing the bytes to a buffer.
///
/// Returns [`Err`] if the data is invalid, lacks the required markers, or the
/// writer returned an error.
pub fn decode<W: io::Write>(mut writer: W, input: &str) -> Result<(), Error> {
    let (skip_last, input) = strip_input(input)?;

    let mut chars = input.chars();
    if let Some(last) = chars.next_back() {
        let last = char_to_bytes(last)?;

        for c in chars {
            let bytes = char_to_bytes(c)?;
            writer.write_all(&bytes)?;
        }

        writer.write_all(match skip_last {
            false => &last[..],
            true => &last[..1],
        })?;
    }

    Ok(())
}

/// Tries to strip a base 65536 input, returning `skip_last` and the stripped
/// input.
fn strip_input(s: &str) -> Result<(bool, &str), Error> {
    // strip the end marker
    let s = s.strip_suffix('&').ok_or(Error::Invalid)?;

    // the start marker is & if the last byte is included
    s.strip_prefix('&')
        .map(|s| (false, s))
        // otherwise, % may be used to indicate the last byte is skipped
        .or_else(|| s.strip_prefix('%').map(|s| (true, s)))
        .filter(|(skip_last, s)| !skip_last || !s.is_empty())
        .ok_or(Error::Invalid)
}

const OFFSET: u32 = 0xE000 - 0xD800;

fn char_to_bytes(c: char) -> Result<[u8; 2], Error> {
    let int = match c {
        '\0'..='\u{D7FF}' => u32::from(c),
        '\u{E000}'..='\u{10FFFF}' => u32::from(c) - OFFSET,
    };

    // char codes greater than 0x107FF would wrap around
    match u16::try_from(int) {
        Ok(i) => Ok(i.to_le_bytes()),
        Err(_) => Err(Error::Invalid),
    }
}

#[must_use]
fn bytes_to_char(bytes: [u8; 2]) -> char {
    let int = u32::from(u16::from_le_bytes(bytes));
    match int {
        // SAFETY: Reverse of `char_to_bytes`.
        0..=0xD7FF => unsafe { char::from_u32_unchecked(int) },
        _ => unsafe { char::from_u32_unchecked(int + OFFSET) },
    }
}
