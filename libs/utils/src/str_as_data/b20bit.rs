//! Encodes bytes as "base 1048576", aka "base 20-bit".
//!
//! Broadly, this encodes bytes in chunks of 5 as 2 characters each. If 1 or 2
//! bytes are left over, half a chunk may be present at the end. Both a start
//! and end marker will be added, depending on the byte count.

use std::{fmt, io};

use super::Error;

/// The maximum byte length a specified count of characters may decode to.
///
/// This can be used to reserve space in a buffer.
pub const fn max_byte_len(char_count: usize) -> usize {
    (char_count - 2) * 5 / 2
}

/// Encodes bytes as "base 20-bit", returning a [`String`] with the result.
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

/// Encodes bytes as "base 20-bit", writing them to a buffer.
///
/// Use [`decode`] to reverse the operation.
///
/// This can only return an [`Err`] if the `writer` does so.
pub fn encode<W: fmt::Write>(mut writer: W, bytes: &[u8]) -> fmt::Result {
    #[inline]
    fn write_chunk<W: fmt::Write>(writer: &mut W, chunk: [u8; 5]) -> fmt::Result {
        let codes = chunk_to_codes(chunk);
        writer.write_char(code_to_char(codes[0]))?;
        writer.write_char(code_to_char(codes[1]))
    }

    #[inline]
    fn write_half_chunk<W: fmt::Write>(writer: &mut W, chunk: [u8; 2]) -> fmt::Result {
        let code = half_chunk_to_code(chunk);
        writer.write_char(code_to_char(code))
    }

    writer.write_char(match bytes.len() % 5 {
        0 => 'A',
        2 | 4 => 'B',
        1 | 3 => 'C',
        _ => unreachable!(),
    })?;

    let mut iter = bytes.chunks_exact(5);
    for chunk in iter.by_ref() {
        // Conversion cannot fail and check is optimized out.
        let chunk = <[u8; 5]>::try_from(chunk).expect("len should be exact");
        write_chunk(&mut writer, chunk)?;
    }

    match *iter.remainder() {
        [] => {},
        [a] => write_half_chunk(&mut writer, [a, 0])?,
        [a, b] => write_half_chunk(&mut writer, [a, b])?,
        // the high 4 bits of `c` are in the 2nd half of the chunk
        [a, b, c] => write_chunk(&mut writer, [a, b, c, 0, 0])?,
        [a, b, c, d] => write_chunk(&mut writer, [a, b, c, d, 0])?,
        _ => unreachable!(),
    }

    writer.write_char('&')
}

/// Decodes a string holding "base 20-bit" data.
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

/// Decodes a string holding "base 20-bit" data, writing the bytes to a buffer.
///
/// Returns [`Err`] if the data is invalid, lacks the required markers, or the
/// writer returned an error.
pub fn decode<W: io::Write>(mut writer: W, input: &str) -> Result<(), Error> {
    let (skip_last, input) = strip_input(input)?;

    let mut chars = input.chars().peekable();
    while let Some(c1) = chars.next() {
        let c1 = char_to_code(c1)?;

        if let Some(c2) = chars.next() {
            let c2 = char_to_code(c2)?;
            let chunk = codes_to_chunk([c1, c2]);
            writer.write_all(match (chars.peek(), skip_last) {
                (Some(_), _) | (None, SkipLast::Zero) => &chunk[..],
                (None, SkipLast::One) => &chunk[..4],
                (None, SkipLast::Two) => &chunk[..3],
            })?;
        } else {
            let chunk = code_to_half_chunk(c1)?;
            writer.write_all(match skip_last {
                // we never encode anything like this
                SkipLast::Zero => return Err(Error::LenMismatch),
                SkipLast::One => &chunk[..2],
                SkipLast::Two => &chunk[..1],
            })?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum SkipLast {
    Zero,
    One,
    Two,
}

/// Tries to strip an input, returning `skip_last` and the stripped input.
fn strip_input(s: &str) -> Result<(SkipLast, &str), Error> {
    // strip the end marker
    let s = s.strip_suffix('&').ok_or(Error::PrefixSuffix)?;

    if let Some(s) = s.strip_prefix('A') {
        return Ok((SkipLast::Zero, s));
    }

    if let Some(s) = s.strip_prefix('B') {
        if s.is_empty() {
            return Err(Error::LenMismatch);
        }

        return Ok((SkipLast::One, s));
    }

    if let Some(s) = s.strip_prefix('C') {
        if s.is_empty() {
            return Err(Error::LenMismatch);
        }

        return Ok((SkipLast::Two, s));
    }

    Err(Error::PrefixSuffix)
}

fn pack_code(prefix: u16, suffix: u8) -> u32 {
    debug_assert!(suffix <= 0xF, "suffix must be at most 4 bits");
    u32::from(prefix) | (u32::from(suffix) << 16)
}

fn half_chunk_to_code(chunk: [u8; 2]) -> u32 {
    pack_code(u16::from_le_bytes([chunk[0], chunk[1]]), 0)
}

fn chunk_to_codes(chunk: [u8; 5]) -> [u32; 2] {
    [
        pack_code(u16::from_le_bytes([chunk[0], chunk[1]]), chunk[2] & 0xF),
        pack_code(
            u16::from_le_bytes([chunk[3], chunk[4]]),
            (chunk[2] & 0xF0) >> 4,
        ),
    ]
}

#[expect(clippy::cast_possible_truncation)]
fn unpack_code(code: u32) -> (u16, u8) {
    debug_assert!(code <= MAX_CODE, "invalid code out of range");
    (code as u16, (code >> 16) as u8)
}

fn code_to_half_chunk(code: u32) -> Result<[u8; 2], Error> {
    let (prefix, suffix) = unpack_code(code);
    if suffix == 0 {
        Ok(prefix.to_le_bytes())
    } else {
        // consider this a length mismatch
        // the code in question is only invalid because the data has the wrong length
        Err(Error::LenMismatch)
    }
}

fn codes_to_chunk(codes: [u32; 2]) -> [u8; 5] {
    let (prefix1, suffix1) = unpack_code(codes[0]);
    let (prefix2, suffix2) = unpack_code(codes[1]);
    let prefix1 = prefix1.to_le_bytes();
    let prefix2 = prefix2.to_le_bytes();
    [
        prefix1[0],
        prefix1[1],
        suffix1 | (suffix2 << 4),
        prefix2[0],
        prefix2[1],
    ]
}

/// Tthe size of the gap in the middle of valid unicode code points.
/// b20bit codes larger than `0xD7FF` have `OFFSET` added to them in the
/// encoded format. When decoding, char codes in the upper range are subtracted
/// by `OFFSET`.
const OFFSET: u32 = 0xE000 - 0xD800;

/// The maximum valid unencoded b20bit code. [`pack_code`] and co. will never
/// return a larger value and, when a larger value is decoded from a char, the
/// input is rejected.
const MAX_CODE: u32 = 0xFFFFF;

fn char_to_code(c: char) -> Result<u32, Error> {
    // the exclusive end of valid char codes.
    // char codes greater than or equal to this will be never be emitted by the
    // encoding, must therefore be invalid, and will be rejected.
    const EX_END: char = char::from_u32(MAX_CODE + OFFSET + 1).unwrap();

    match c {
        '\0'..='\u{D7FF}' => Ok(u32::from(c)),
        '\u{E000}'..EX_END => Ok(u32::from(c) - OFFSET),
        EX_END..='\u{10FFFF}' => Err(Error::ContentFormat),
    }
}

fn code_to_char(code: u32) -> char {
    match code {
        // SAFETY: reverse of `char_to_code`.
        // every potential char we can construct here will be in range
        0..=0xD7FF => unsafe { char::from_u32_unchecked(code) },
        0xD800..=MAX_CODE => unsafe { char::from_u32_unchecked(code + OFFSET) },
        // packed codes never exceed `MAX_CODE`
        _ => unreachable!("invalid packed code"),
    }
}
