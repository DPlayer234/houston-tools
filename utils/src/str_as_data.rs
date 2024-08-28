crate::define_simple_error!(
    /// Error decoding base256 data in [`from_b256`].
    Base256Error(()):
    "base256 data is invalid"
);

crate::define_simple_error!(
    /// Error decoding base65536 data in [`from_b65536`].
    Base65536Error(()):
    "base65536 data is invalid"
);

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
/// Byte will be paired. The combined value of each pair will mapped to UTF-8 characters
/// and the sequence is then joined. A marker for whether the input sequence had an odd
/// amount of bytes will be stored.
///
/// The sequence will be prefixed with a header character and ends with `&`.
#[must_use]
pub fn to_b65536(bytes: &[u8]) -> String {
    // A little testing indicates that the output
    // generally takes 25%-50% more bytes.
    let expected_size = 2 + bytes.len() + (bytes.len() >> 1);

    let mut result = String::with_capacity(expected_size);

    // Note that this '&' is unsafely assumed
    // to be present later in this function.
    result.push('&');

    let mut iter = bytes.chunks_exact(2);
    for chunk in iter.by_ref() {
        // Conversion cannot fail and check is optimized out.
        let chunk = <[u8; 2]>::try_from(chunk).unwrap();
        result.push(bytes_to_char(chunk));
    }

    if let &[last] = iter.remainder() {
        let chunk = [last, 0];
        result.push(bytes_to_char(chunk));
        unsafe {
            // SAFETY: result starts with '&'.
            // We can replace this ASCII character with another.
            *result.as_bytes_mut().get_unchecked_mut(0) = b'%';
        }
    }

    result.push('&');
    result
}

/// Reverses the operation done by [`to_b65536`].
///
/// If the data is invalid or lacks the required markers, returns an error.
pub fn from_b65536(str: &str) -> Result<Vec<u8>, Base65536Error> {
    let (skip_last, str) = str
        // strip the end marker
        .strip_suffix('&')
        // strip the start marker
        .and_then(|s| {
            // the start marker is & if the last byte is included
            s.strip_prefix('&').map(|s| (false, s))
            // otherwise, % may be used to indicate the last byte is skipped
            .or_else(|| s.strip_prefix('%').map(|s| (true, s)))
        })
        .ok_or(Base65536Error(()))?;

    let mut result = Vec::new();
    for c in str.chars() {
        let bytes = char_to_bytes(c)?;
        result.extend(bytes);
    }

    if skip_last && result.pop().is_none() {
        return Err(Base65536Error(()));
    }

    Ok(result)
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
        Err(_) => Err(Base65536Error(())),
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
    fn invalid_char_b65535_fails() {
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
