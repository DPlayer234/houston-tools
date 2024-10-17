use super::Error;

/// Converts the bytes to "base 256".
///
/// Each byte will be mapped to the UTF-8 character with the equivalent code.
///
/// The sequence will be prefixed with `#` and ends with `&`.
#[must_use]
pub fn to_b256(bytes: &[u8]) -> String {
    let expected_size = 2 + bytes.len() + (bytes.len() >> 1);
    let mut result = String::with_capacity(expected_size);

    encode_b256(&mut result, bytes)
        .expect("write to String cannot fail");

    result
}

/// Encodes the bytes to "base 256", writing them to a buffer.
///
/// See [`to_b256`] for more information.
///
/// This can only return an [`Err`] if the `writer` does so.
pub fn encode_b256<W: std::fmt::Write>(mut writer: W, bytes: &[u8]) -> std::fmt::Result {
    writer.write_char('#')?;
    for b in bytes {
        writer.write_char(char::from(*b))?;
    }

    writer.write_char('&')
}

/// Reverses the operation done by [`to_b256`].
///
/// If the data is invalid or lacks the required markers, returns an error.
pub fn from_b256(input: &str) -> Result<Vec<u8>, Error> {
    let expected_size = input.len().saturating_sub(2);
    let mut result = Vec::with_capacity(expected_size);

    decode_b256(&mut result, input)?;
    Ok(result)
}

/// Reverses the operation done by [`to_b256`], writing to a given buffer.
///
/// If the data is invalid or lacks the required markers, returns an error.
pub fn decode_b256<W: std::io::Write>(mut writer: W, input: &str) -> Result<(), Error> {
    let input = try_strip_b256_input(input)?;

    for c in input.chars() {
        let byte = u8::try_from(c).map_err(|_| Error::Invalid)?;
        writer.write_all(&[byte])?;
    }

    Ok(())
}

fn try_strip_b256_input(input: &str) -> Result<&str, Error> {
    input
        // strip the start marker
        .strip_prefix('#')
        // strip the end marker
        .and_then(|s| s.strip_suffix('&'))
        .ok_or(Error::Invalid)
}
