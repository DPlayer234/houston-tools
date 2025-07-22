//! Encodes bytes as "base 1048576", aka "base 20-bit".
//!
//! Broadly, this encodes bytes in chunks of 5 as 2 characters each. If 1 or 2
//! bytes are left over, half a chunk may be present at the end. Both a start
//! and end marker will be added, depending on the byte count.
//!
//! The exact format is as follows:
//!
//! - The prefix is added, based on the byte count modulo 5: `A` if it is 0, `B`
//!   if is it 2 or 4, `C` if is 1 or 3.
//! - The byte slice is chunked into 5-byte chunks. Each chunk is transformed
//!   into a [`char`] pair as described below. The last chunk is padded with
//!   null-bytes to become 5 bytes; if it was originally 2 or less bytes, only
//!   the first [`char`] is added.
//! - The suffix is added: It is always `&`.
//!
//! Each 5 byte group will be encoded into [`char`] pairs via the following
//! transformation:
//!
//! - Let the byte pattern be: `0x12, 0x34, 0x56, 0x78, 0x90`
//! - The nibbles are rearranged into 2 codes as: `0x63412, 0x59078`
//! - Each code is mapped to a [`char`]. For codes between `0x0` to `0xD7FF` the
//!   [`char`] with the code point equal to the value is used; for codes between
//!   `0xD800` to `0xFFFFF` the [`char`] with the code point equal to the value
//!   plus `0x800` is used instead.
//! - This example would thefore encode to the string `\u{63C12}\u{59878}`.
//!
//! These rules ensure only valid unicode code points are in the output.
//!
//! Decoding applies these rules in reverse, with only [`char`] codes in the
//! range `0x0` to `0xD7FF` and `0xE000` to `0x1007FF` being allowed.
//!
//! The prefix indicates how many bytes to remove from the end of the decoded
//! output. If the string had an odd amount of [`char`] values, the last
//! [`char`] is treated as if it encoded 3 bytes.
//!
//! - If the prefix is `A`, no bytes are trimmed. The entire string is used.
//! - If the prefix is `B`, 1 byte are trimmed from the end.
//! - If the prefix is `C`, 2 bytes are trimmed from the end.
//!
//! # Encoding Gap
//!
//! This definition leaves a gap in the encoding: An odd [`char`] count with
//! payload prefix `A`. This would imply that the last chunk of the input was 3
//! bytes long. This could only be correct if the 3rd byte of said chunk is in
//! range `0x0` to `0xF`, which isn't deemed worth it, so that currently isn't
//! encoded and it instead falls back to padding to a 5 byte chunk and using
//! prefix `C`.
//!
//! The current decoder implemented here does not support decoding this case
//! since it has minimal impacts on bandwidth and adds unneeded complexity to
//! the code.
//!
//! # FAQ
//!
//! **Q: Why?**\
//! A: Why not.
//!
//! **Q: What's with the weird nibble ordering?**\
//! A: I didn't think about it while writing the code.
//!
//! **Q: Why does the second [`char`] of the last chunk get omitted if that
//! chunk was 2 or less bytes, even though the remaining [`char`] is treated as
//! if it encoded 3 bytes when read back?**\
//! A: Initially, the code actually encoded 3 bytes, even though that could't
//! work as it was written, so it was changed. There is also the future compat
//! spec gap that may be implemented at a later date.

use std::{fmt, io};

use super::Error;

/// Byte count of a full chunk.
const FULL: usize = 5;

/// Byte count of a half chunk.
const HALF: usize = FULL / 2;

/// Amount of [`char`]s a full chunk is packed into.
const PACK: usize = 2;

/// Extra bytes in the encoded format that don't encode source data directly.
///
/// This represents the 1 byte for the start and end marker each.
const EXTRA: usize = 2;

/// The maximum byte length a specified count of characters may decode to.
///
/// This can be used to reserve space in a buffer.
pub const fn max_byte_len(char_count: usize) -> usize {
    (char_count - EXTRA) * FULL / PACK
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
    let expected_size = EXTRA + (bytes.len() << 1);
    let mut result = String::with_capacity(expected_size);

    encode(&mut result, bytes).expect("write to String cannot fail");

    result
}

/// Encodes bytes as "base 20-bit", writing them to a buffer.
///
/// Use [`decode`] to reverse the operation.
///
/// # Errors
///
/// Returns [`Err`] if and only if `writer` returns [`Err`].
pub fn encode<W: fmt::Write>(mut writer: W, bytes: &[u8]) -> fmt::Result {
    #[inline]
    fn write_chunk<W: fmt::Write>(writer: &mut W, chunk: [u8; FULL]) -> fmt::Result {
        let codes = chunk_to_codes(chunk);
        writer.write_char(code_to_char(codes[0]))?;
        writer.write_char(code_to_char(codes[1]))
    }

    #[inline]
    fn write_half_chunk<W: fmt::Write>(writer: &mut W, chunk: [u8; HALF]) -> fmt::Result {
        let code = half_chunk_to_code(chunk);
        writer.write_char(code_to_char(code))
    }

    writer.write_char(match bytes.len() % FULL {
        0 => 'A',
        2 | 4 => 'B',
        1 | 3 => 'C',
        _ => unreachable!(),
    })?;

    let (chunks, remainder) = bytes.as_chunks::<FULL>();
    for &chunk in chunks {
        write_chunk(&mut writer, chunk)?;
    }

    match *remainder {
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
/// # Errors
///
/// Returns [`Err`] if the data is invalid or lacks the required markers.
pub fn from_str(input: &str) -> Result<Vec<u8>, Error> {
    // Extending the logic in `to_string`, less than ~130% is also common.
    // This almost always has enough space and rarely leads to more than
    // half the capacity going entirely unused.
    let expected_size = input.len().saturating_sub(EXTRA);
    let mut result = Vec::with_capacity(expected_size);

    decode(&mut result, input)?;
    Ok(result)
}

/// Decodes a string holding "base 20-bit" data, writing the bytes to a buffer.
///
/// # Errors
///
/// Returns [`Err`] if the data is invalid, lacks the required markers, or
/// `writer` returns [`Err`].
pub fn decode<W: io::Write>(mut writer: W, input: &str) -> Result<(), Error> {
    let (skip_last, input) = strip_input(input)?;

    // note: we don't check that the skipped bytes are zero
    let mut chars = input.chars().peekable();
    while let Some(c1) = chars.next() {
        let c1 = char_to_code(c1)?;

        if let Some(c2) = chars.next() {
            let c2 = char_to_code(c2)?;
            let chunk = codes_to_chunk([c1, c2]);
            writer.write_all(match (chars.peek(), skip_last) {
                // if there are more bytes, take the full chunk
                // if at the end, skip the last bytes as indicated by the header
                (Some(_), _) | (None, SkipLast::Zero) => &chunk[..],
                (None, SkipLast::One) => &chunk[..4],
                (None, SkipLast::Two) => &chunk[..3],
            })?;
        } else {
            // half chunks are treated as if they were 3 bytes, but they can only fully
            // encode 2 bytes. subsequently, we treat this `[u8; 2]` half-chunk as if it was
            // followed by a phantom zero-byte. zero-skip is invalid due to this.
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
///
/// # Errors
///
/// Returns [`Err`] if the start or end marker is incorrect, or the prefix
/// cannot possibly match the data length.
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

/// Packs a 16-bit prefix and 4-bit suffix into a 20-bit code.
fn pack_code(prefix: u16, suffix: u8) -> u32 {
    debug_assert!(suffix <= 0xF, "suffix must be at most 4 bits");
    u32::from(prefix) | (u32::from(suffix) << 16)
}

/// Converts a half-chunk into a 20-bit code.
fn half_chunk_to_code(chunk: [u8; HALF]) -> u32 {
    pack_code(u16::from_le_bytes([chunk[0], chunk[1]]), 0)
}

/// Converts a full chunk into two 20-bit codes.
///
/// The nibbles of the 3rd byte are encoded as the suffixes.
fn chunk_to_codes(chunk: [u8; FULL]) -> [u32; PACK] {
    [
        pack_code(u16::from_le_bytes([chunk[0], chunk[1]]), chunk[2] & 0xF),
        pack_code(
            u16::from_le_bytes([chunk[3], chunk[4]]),
            (chunk[2] & 0xF0) >> 4,
        ),
    ]
}

/// Unpacks a 20-bit code back into 16-bit prefix and a 4-bit suffix.
#[expect(clippy::cast_possible_truncation)]
fn unpack_code(code: u32) -> (u16, u8) {
    debug_assert!(code <= MAX_CODE, "invalid code out of range");
    (code as u16, (code >> 16) as u8)
}

/// Converts a 20-bit code to a half-chunk.
///
/// # Errors
///
/// Returns [`Err`] if the suffix is non-zero.
fn code_to_half_chunk(code: u32) -> Result<[u8; HALF], Error> {
    let (prefix, suffix) = unpack_code(code);
    if suffix == 0 {
        Ok(prefix.to_le_bytes())
    } else {
        // consider this a length mismatch
        // the code in question is only invalid because the data has the wrong length
        Err(Error::LenMismatch)
    }
}

/// Converts two 20-bit codes into a full chunk.
fn codes_to_chunk(codes: [u32; PACK]) -> [u8; FULL] {
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

/// The size of the gap in the middle of valid unicode code points.
/// b20bit codes larger than `0xD7FF` have `OFFSET` added to them in the
/// encoded format. When decoding, char codes in the upper range are subtracted
/// by `OFFSET`.
const OFFSET: u32 = 0xE000 - 0xD800;

/// The maximum valid unencoded b20bit code. [`pack_code`] and co. will never
/// return a larger value and, when a larger value is decoded from a char, the
/// input is rejected.
const MAX_CODE: u32 = 0xFFFFF;

/// Converts a [`char`] to a 20-bit code.
///
/// # Errors
///
/// Returns [`Err`] if the character code would need more than 20 bits.
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

/// Converts a 20-bit code to a [`char`].
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
