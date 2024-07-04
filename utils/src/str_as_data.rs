crate::define_simple_error!(Base256Error: "base256 data is invalid");
crate::define_simple_error!(Base65536Error: "base65536 data is invalid");

#[must_use]
pub fn to_b256(bytes: &[u8]) -> String {
    use std::iter::once;

    let input = bytes.into_iter().map(|b| char::from(*b));
    once('#').chain(input).chain(once('&')).collect()
}

#[must_use]
pub fn from_b256(str: &str) -> Result<Vec<u8>, Base256Error> {
    let str = str
        // strip the start marker
        .strip_prefix('#')
        // strip the end marker
        .and_then(|s| s.strip_suffix('&'))
        .ok_or(Base256Error)?;

    str.chars().map(u8::try_from)
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|_| Base256Error)
}

#[must_use]
pub fn to_b65536(bytes: &[u8]) -> String {
    // A little testing indicates that the output
    // generally takes 25%-50% more bytes.
    let expected_size = 2 + bytes.len() + bytes.len() >> 1;

    let mut result = String::with_capacity(expected_size);

    // Note that this '&' is unsafely assumed
    // to be present later in this function.
    result.push('&');

    let mut iter = bytes.chunks_exact(2);
    while let Some(chunk) = iter.next() {
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

#[must_use]
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
        .ok_or(Base65536Error)?;

    let mut result: Vec<u8> = str
        .chars()
        .flat_map(char_to_bytes)
        .collect();

    if skip_last {
        if result.len() == 0 {
            return Err(Base65536Error);
        }

        result.remove(result.len() - 1);
    }

    Ok(result)
}

const OFFSET: u32 = 0xE000 - 0xD800;

fn char_to_bytes(c: char) -> [u8; 2] {
    let int = match c {
        '\0' ..= '\u{D7FF}' => u32::from(c),
        '\u{E000}' ..= '\u{10FFFF}' => u32::from(c) - OFFSET,
    };

    (int as u16).to_le_bytes()
}

fn bytes_to_char(bytes: [u8; 2]) -> char {
    // SAFETY: Reverse of `char_to_bytes`.
    let int = u16::from_le_bytes(bytes) as u32;
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
        assert!(std::mem::size_of::<u16>() == 2);

        const DATA_SIZE: usize = (u16::MAX as usize) * 2;
        const fn create_data() -> [u8; DATA_SIZE] {
            let mut result = [0u8; DATA_SIZE];
            let mut index = 0usize;
            while index < result.len() {
                let num = (index / 2) as u16;
                result[index] = num as u8;
                result[index + 1] = (num >> 8) as u8;
                index += 2;
            }

            result
        }

        &create_data()
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
    fn round_trip_b65535_even() {
        round_trip_core(
            DATA,
            to_b65536,
            from_b65536
        );
    }

    #[test]
    fn round_trip_b65535_odd() {
        round_trip_core(
            &DATA[1..],
            to_b65536,
            from_b65536
        );
    }

    fn round_trip_core<E: std::fmt::Debug>(bytes: &[u8], encode: impl FnOnce(&[u8]) -> String, decode: impl FnOnce(&str) -> Result<Vec<u8>, E>) {
        let encoded = black_box(encode(&bytes));
        println!("encoded[{}]", encoded.chars().count());

        let back = decode(&encoded).expect("decoding failed");

        assert_eq!(back.as_slice(), bytes);
    }
}
