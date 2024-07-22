/// Represents a [`str`] with a fixed length and ownership semantics.
/// Essentially, it is to [`&str`](str) what `[T; N]` is to `&[T]`.
///
/// `LEN` represents the size in bytes, using the same semantics as [`str::len`].
///
/// Like [`str`], it may only contain valid UTF-8 bytes.
///
/// Generally, [`String`] is more useful but this is can be useful
/// for working with strings in a const context.
///
// Note: These derives are fine since `str` itself only delegates to `as_bytes`.
//       `Debug` and `Hash` are manually implemented to delegate to `as_str`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct InlineStr<const LEN: usize>([u8; LEN]);

impl<const LEN: usize> InlineStr<LEN> {
    /// Converts an array to an [`InlineStr`].
    ///
    /// This has the same semantics as [`std::str::from_utf8`].
    pub const fn from_utf8(bytes: [u8; LEN]) -> Result<Self, std::str::Utf8Error> {
        match std::str::from_utf8(&bytes) {
            Ok(..) => Ok(unsafe {
                // SAFETY: from_utf8 checks validity
                Self::from_utf8_unchecked(bytes)
            }),
            Err(err) => Err(err)
        }
    }

    /// Converts an array to an [`InlineStr`] without checking the string contains valid UTF-8.
    ///
    /// Refer to [`std::str::from_utf8`] for exact semantics.
    ///
    /// # Safety
    ///
    /// All bytes passed in must be valid UTF-8.
    pub const unsafe fn from_utf8_unchecked(bytes: [u8; LEN]) -> Self {
        // SAFETY: Caller has to ensure the bytes are valid UTF-8
        Self(bytes)
    }

    /// Always returns `LEN`.
    pub const fn len(&self) -> usize {
        LEN
    }

    /// Converts this value to a [`str`] slice.
    pub const fn as_str(&self) -> &str {
        unsafe {
            // SAFETY: Only constructed with valid UTF-8
            std::str::from_utf8_unchecked(&self.0)
        }
    }

    /// Converts this value to a mutable [`str`] slice.
    pub fn as_mut_str(&mut self) -> &mut str {
        unsafe {
            // SAFETY: Only constructed with valid UTF-8
            std::str::from_utf8_unchecked_mut(&mut self.0)
        }
    }

    /// Converts a string to a byte array.
    pub const fn as_bytes(&self) -> &[u8; LEN] {
        &self.0
    }

    /// Converts a mutable string to a mutable byte array.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the contents of the array are valid UTF-8 before the borrow ends
    /// and the underlying data is used as a [`str`].
    ///
    /// Also refer to [`str::as_bytes_mut`].
    pub unsafe fn as_bytes_mut(&mut self) -> &mut [u8; LEN] {
        &mut self.0
    }
}

impl Default for InlineStr<0> {
    fn default() -> Self {
        Self([])
    }
}

impl<const LEN: usize> std::ops::Deref for InlineStr<LEN> {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl<const LEN: usize> std::ops::DerefMut for InlineStr<LEN> {
    fn deref_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<const LEN: usize> std::borrow::Borrow<str> for InlineStr<LEN> {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<const LEN: usize> std::borrow::BorrowMut<str> for InlineStr<LEN> {
    fn borrow_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<const LEN: usize> std::fmt::Display for InlineStr<LEN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_str(), f)
    }
}

impl<const LEN: usize> std::fmt::Debug for InlineStr<LEN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl<const LEN: usize> std::hash::Hash for InlineStr<LEN> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(self.as_str(), state)
    }
}
