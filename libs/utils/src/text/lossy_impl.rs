/// Appends possibly non-UTF-8 bytes to this string.
///
/// Valid UTF-8 sequences are appended as is, while invalid sequences are
/// replaced by [`char::REPLACEMENT_CHARACTER`].
///
/// # Examples
///
/// ```
/// use utils::text::push_str_lossy;
///
/// // "Hello " + invalid UTF-8 + "World!"
/// let buf = b"Hello \xF0\x90\x80World!";
/// let mut target = String::new();
/// push_str_lossy(&mut target, buf);
/// assert_eq!(target, "Hello �World!");
/// ```
pub fn push_str_lossy(target: &mut String, buf: &[u8]) {
    for chunk in buf.utf8_chunks() {
        target.push_str(chunk.valid());

        if !chunk.invalid().is_empty() {
            target.push(char::REPLACEMENT_CHARACTER);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::push_str_lossy;

    #[test]
    fn test_push_str_lossy_only_valid() {
        let buf = b"Hello World!";
        let mut target = String::new();
        push_str_lossy(&mut target, buf);
        assert_eq!(target, "Hello World!");
    }

    #[test]
    fn test_push_str_lossy_only_invalid() {
        // invalid UTF-8
        let buf = b"\x80\x80\x80\x80";
        let mut target = String::new();
        push_str_lossy(&mut target, buf);
        assert_eq!(target, "����");
    }

    #[test]
    fn test_push_str_lossy_mixed() {
        // "Valid " + invalid UTF-8 + "Invalid " + valid UTF-8 (snowman)
        let buf = b"Valid \xF0\x90\x80Invalid \xE2\x98\x83";
        let mut target = String::new();
        push_str_lossy(&mut target, buf);
        assert_eq!(target, "Valid �Invalid ☃");
    }
}
