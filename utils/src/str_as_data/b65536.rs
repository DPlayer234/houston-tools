use std::{fmt, io};

use super::Error;

/// Converts the bytes to "base 65535".
///
/// Bytes will be paired. The combined value of each pair will mapped to UTF-8
/// characters and the sequence is then joined. A marker for whether the input
/// sequence had an odd amount of bytes will be stored.
///
/// The sequence will be prefixed with a header character and ends with `&`.
#[must_use]
pub fn to_b65536(bytes: &[u8]) -> String {
    // Testing indicates more than 100% is normal, usually about ~130%.
    // But more is still common and more than 200% is rare, so we go for that.
    let expected_size = 2 + (bytes.len() << 1);
    let mut result = String::with_capacity(expected_size);

    encode_b65536(&mut result, bytes).expect("write to String cannot fail");

    result
}

/// Encodes the bytes to "base 65535", writing them to a buffer.
///
/// See [`to_b65536`] for more information.
///
/// This can only return an [`Err`] if the `writer` does so.
pub fn encode_b65536<W: fmt::Write>(mut writer: W, bytes: &[u8]) -> fmt::Result {
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

/// Reverses the operation done by [`to_b65536`].
///
/// If the data is invalid or lacks the required markers, returns an error.
pub fn from_b65536(input: &str) -> Result<Vec<u8>, Error> {
    // Extending the logic in `to_b65536`, less than ~130% is also common.
    // This almost always has enough space and rarely leads to more than
    // half the capacity going entirely unused.
    let expected_size = input.len().saturating_sub(2);
    let mut result = Vec::with_capacity(expected_size);

    decode_b65536(&mut result, input)?;
    Ok(result)
}

/// Reverses the operation done by [`to_b65536`], writing to a given buffer.
///
/// If the data is invalid or lacks the required markers, returns an error.
pub fn decode_b65536<W: io::Write>(mut writer: W, input: &str) -> Result<(), Error> {
    let (skip_last, input) = try_strip_b65536_input(input)?;

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
fn try_strip_b65536_input(s: &str) -> Result<(bool, &str), Error> {
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
