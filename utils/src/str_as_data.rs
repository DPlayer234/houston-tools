//! Provides ways to encode binary data as valid UTF-8 strings and
//! convert those strings back into binary data.
//!
//! To avoid trimming of white-space at the start and end of strings,
//! every string output has delimiters added.
//!
//! The current supported formats are:
//!
//! ## Base 256
//!
//! Via [`to_b256`] and [`from_b256`]:
//! Encodes each byte as one [`char`] of the output with the equivalent code point value.
//!
//! ## Base 65536:
//!
//! Via [`to_b65536`] and [`from_b65536`]:
//! Encodes pairs of bytes as one [`char`] of the output with a unique code point for each possible input.
//!
//! If you wish to write to existing buffers, you may also use [`encode_b65536`] and [`decode_b65536`].

crate::define_simple_error!(
    /// Error decoding base 256 data in [`from_b256`].
    Base256Error(()):
    "base256 data is invalid"
);

crate::define_simple_error!(
    /// Error decoding base 65536 data in [`from_b65536`].
    Base65536Error(ErrorReason):
    s => "base65536: {}", s.0
);

/// The decoding error reason.
#[derive(Debug)]
#[non_exhaustive]
pub enum ErrorReason {
    /// The data was invalid.
    Invalid,
    /// The written buffer returned an error.
    Io(std::io::Error),
}

impl Base65536Error {
    const fn invalid() -> Self {
        Self(ErrorReason::Invalid)
    }

    /// Gets the underlying error reason.
    pub fn kind(&self) -> &ErrorReason {
        &self.0
    }
}

impl std::fmt::Display for ErrorReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid => f.write_str("data is invalid"),
            Self::Io(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

/// Converts the bytes to "base 256".
///
/// Each byte will be mapped to the UTF-8 character with the equivalent code.
///
/// The sequence will be prefixed with `#` and ends with `&`.
#[must_use]
pub fn to_b256(bytes: &[u8]) -> String {
    use std::iter::once;

    let input = bytes.iter().map(|b| char::from(*b));
    once('#').chain(input).chain(once('&')).collect()
}

/// Reverses the operation done by [`to_b256`].
///
/// If the data is invalid or lacks the required markers, returns an error.
pub fn from_b256(str: &str) -> Result<Vec<u8>, Base256Error> {
    let str = str
        // strip the start marker
        .strip_prefix('#')
        // strip the end marker
        .and_then(|s| s.strip_suffix('&'))
        .ok_or(Base256Error(()))?;

    str.chars().map(u8::try_from)
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|_| Base256Error(()))
}

/// Converts the bytes to "base 65535".
///
/// Bytes will be paired. The combined value of each pair will mapped to UTF-8 characters
/// and the sequence is then joined. A marker for whether the input sequence had an odd
/// amount of bytes will be stored.
///
/// The sequence will be prefixed with a header character and ends with `&`.
#[must_use]
pub fn to_b65536(bytes: &[u8]) -> String {
    // Testing indicates more than 100% is normal, usually about ~130%.
    // But more is still common and more than 200% is rare, so we go for that.
    let expected_size = 2 + (bytes.len() << 1);
    let mut result = String::with_capacity(expected_size);

    encode_b65536(&mut result, bytes)
        .expect("write to String cannot fail");

    result
}

/// Encodes the bytes to "base 65535", writing them to a buffer.
///
/// See [`to_b65536`] for more information.
///
/// This can only return an [`Err`] if the `writer` does so.
pub fn encode_b65536<W: std::fmt::Write>(mut writer: W, bytes: &[u8]) -> std::fmt::Result {
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
pub fn from_b65536(input: &str) -> Result<Vec<u8>, Base65536Error> {
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
pub fn decode_b65536<W: std::io::Write>(mut writer: W, input: &str) -> Result<(), Base65536Error> {
    const fn io_err(err: std::io::Error) -> Base65536Error {
        Base65536Error(ErrorReason::Io(err))
    }

    let (skip_last, input) = try_strip_b65536_input(input)?;

    let mut chars = input.chars();
    if let Some(last) = chars.next_back() {
        let last = char_to_bytes(last)?;

        for c in chars {
            let bytes = char_to_bytes(c)?;
            writer.write_all(&bytes).map_err(io_err)?;
        }

        writer.write_all(match skip_last {
            false => &last[..],
            true => &last[..1],
        }).map_err(io_err)?;
    }

    Ok(())
}

/// Tries to strip a base 65536 input, returning `skip_last` and the stripped input.
fn try_strip_b65536_input(str: &str) -> Result<(bool, &str), Base65536Error> {
    str
        // strip the end marker
        .strip_suffix('&')
        // strip the start marker
        .and_then(|s| {
            // the start marker is & if the last byte is included
            s.strip_prefix('&').map(|s| (false, s))
            // otherwise, % may be used to indicate the last byte is skipped
            .or_else(|| s.strip_prefix('%').map(|s| (true, s)))
        })
        .filter(|(skip_last, str)| !skip_last || !str.is_empty())
        .ok_or(Base65536Error::invalid())
}

const OFFSET: u32 = 0xE000 - 0xD800;

fn char_to_bytes(c: char) -> Result<[u8; 2], Base65536Error> {
    let int = match c {
        '\0' ..= '\u{D7FF}' => u32::from(c),
        '\u{E000}' ..= '\u{10FFFF}' => u32::from(c) - OFFSET,
    };

    // char codes greater than 0x107FF would wrap around
    match u16::try_from(int) {
        Ok(i) => Ok(i.to_le_bytes()),
        Err(_) => Err(Base65536Error::invalid()),
    }
}

#[must_use]
fn bytes_to_char(bytes: [u8; 2]) -> char {
    // SAFETY: Reverse of `char_to_bytes`.
    let int = u32::from(u16::from_le_bytes(bytes));
    match int {
        0 ..= 0xD7FF => unsafe { char::from_u32_unchecked(int) },
        _ => unsafe { char::from_u32_unchecked(int + OFFSET) },
    }
}

#[cfg(test)]
mod test {
    use std::hint::black_box;
    use super::*;

    static DATA: &[u8] = {
        const MAX: usize = u16::MAX as usize;
        const fn create_data() -> [u16; MAX] {
            let mut result = [0u16; MAX];
            let mut index = 0usize;

            #[allow(clippy::cast_possible_truncation)]
            while index < result.len() {
                result[index] = index as u16;
                index += 1;
            }

            result
        }

        unsafe {
            crate::mem::as_bytes(&create_data())
        }
    };

    #[test]
    fn round_trip_b256() {
        round_trip_core(
            DATA,
            to_b256,
            from_b256
        );
    }

    #[test]
    fn round_trip_b65536_even() {
        round_trip_core(
            DATA,
            to_b65536,
            from_b65536
        );
    }

    #[test]
    fn round_trip_b65536_odd() {
        round_trip_core(
            &DATA[1..],
            to_b65536,
            from_b65536
        );
    }

    #[test]
    fn min_b256() {
        let encoded = black_box("#\u{0078}&");
        let back = from_b256(encoded).expect("decoding failed");

        assert_eq!(back.as_slice(), &[0x78]);
    }

    #[test]
    fn min_b65536() {
        let encoded = black_box("&\u{1020}&");
        let back = from_b65536(encoded).expect("decoding failed");

        assert_eq!(back.as_slice(), &[0x20, 0x10]);
    }

    #[test]
    fn invalid_char_b256_fails() {
        let encoded = black_box("%\u{10800}&");
        from_b65536(encoded).expect_err("U+10800 is out of range");
    }

    #[test]
    fn invalid_char_b65536_fails() {
        let encoded = black_box("#\u{0100}&");
        from_b256(encoded).expect_err("U+256 is out of range");
    }

    fn round_trip_core<E: std::fmt::Debug>(bytes: &[u8], encode: impl FnOnce(&[u8]) -> String, decode: impl FnOnce(&str) -> Result<Vec<u8>, E>) {
        let encoded = black_box(encode(bytes));
        println!("encoded[{}]", encoded.chars().count());

        let back = decode(&encoded).expect("decoding failed");

        assert_eq!(back.as_slice(), bytes);
    }
}
