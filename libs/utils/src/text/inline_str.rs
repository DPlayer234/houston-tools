use std::borrow::{Borrow, BorrowMut};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem::transmute;
use std::ops::{Deref, DerefMut};
use std::str::Utf8Error;

/// Represents a [`str`] with a fixed length and ownership semantics.
/// Essentially, it is to [`&str`](str) what `[T; LEN]` is to `&[T]`.
///
/// `LEN` represents the size in bytes, using the same semantics as
/// [`str::len`].
///
/// Like [`str`], it may only contain valid UTF-8 bytes.
///
/// Generally, [`String`] is more useful but this is can be useful for working
/// with strings in a const context.
// Note: These derives are fine since `str` itself only delegates to `as_bytes` for `Eq` and `Ord`.
// `Debug` and `Hash` are manually implemented to delegate to `as_str` to give the right `Borrow`
// semantics.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct InlineStr<const LEN: usize>([u8; LEN]);

/// Converting to [`InlineStr`] from [`str`] failed because of a length
/// mismatch.
#[derive(Debug, thiserror::Error)]
#[error("length of input does not match result length")]
pub struct FromStrError(());

impl<const LEN: usize> InlineStr<LEN> {
    /// Converts an array to an [`InlineStr`].
    ///
    /// This has the same semantics as [`str::from_utf8`].
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the slice is not valid UTF-8.
    pub const fn from_utf8(bytes: [u8; LEN]) -> Result<Self, Utf8Error> {
        match str::from_utf8(&bytes) {
            // SAFETY: `from_utf8` returns Ok only on valid UTF-8
            Ok(_) => Ok(unsafe { Self::from_utf8_unchecked(bytes) }),
            Err(err) => Err(err),
        }
    }

    /// Converts an array to an [`InlineStr`] without checking the string
    /// contains valid UTF-8.
    ///
    /// Refer to [`str::from_utf8`] for exact semantics.
    ///
    /// # Safety
    ///
    /// All bytes passed in must be valid UTF-8.
    #[must_use]
    pub const unsafe fn from_utf8_unchecked(bytes: [u8; LEN]) -> Self {
        // caller has to ensure the bytes are valid UTF-8. other unsafe code relies on
        // the bytes stored within to be valid UTF-8.
        Self(bytes)
    }

    /// Creates a reference to an [`InlineStr`] from a [`&str`](str). The
    /// returned reference points to the same memory and must have the same
    /// length.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the length of the slice does not match `N`.
    pub const fn from_str(str: &str) -> Result<&Self, FromStrError> {
        match str.as_bytes().as_array() {
            // SAFETY: `InlineStr<LEN>` is a transparent wrapper around `[u8; LEN]`
            // and `array` is derived from a `str`, so it must be valid UTF-8.
            Some(array) => Ok(unsafe { transmute::<&[u8; LEN], &Self>(array) }),
            None => Err(FromStrError(())),
        }
    }

    /// Always returns `LEN`.
    #[must_use]
    pub const fn len(&self) -> usize {
        LEN
    }

    /// Returns `LEN == 0`.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        LEN == 0
    }

    /// Converts this value to a [`str`] slice.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        // SAFETY: `self` must have been constructed with valid UTF-8
        unsafe { str::from_utf8_unchecked(&self.0) }
    }

    /// Converts this value to a mutable [`str`] slice.
    #[must_use]
    pub const fn as_mut_str(&mut self) -> &mut str {
        // SAFETY: `self` must have been constructed with valid UTF-8
        unsafe { str::from_utf8_unchecked_mut(&mut self.0) }
    }

    /// Converts a string to a byte array.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; LEN] {
        &self.0
    }

    /// Converts a mutable string to a mutable byte array.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the contents of the array are valid UTF-8
    /// before the borrow ends and the underlying data is used as a [`str`].
    ///
    /// Also refer to [`str::as_bytes_mut`].
    #[must_use]
    pub const unsafe fn as_bytes_mut(&mut self) -> &mut [u8; LEN] {
        &mut self.0
    }

    /// Joins two fixed-size strings into a new fixed-size string.
    ///
    /// # Panics
    ///
    /// Panics if the FINAL length doesn't match the total length of the inputs.
    /// This will happen at compile time rather than runtime.
    #[must_use]
    pub const fn join<const OTHER: usize, const FINAL: usize>(
        self,
        other: InlineStr<OTHER>,
    ) -> InlineStr<FINAL> {
        const {
            assert!(
                LEN + OTHER == FINAL,
                "length of inputs doesn't match result length"
            );
        }
        super::private::join_str_const(&[self.as_str(), other.as_str()])
    }
}

impl<const LEN: usize> Deref for InlineStr<LEN> {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl<const LEN: usize> DerefMut for InlineStr<LEN> {
    fn deref_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<const LEN: usize> Borrow<str> for InlineStr<LEN> {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<const LEN: usize> BorrowMut<str> for InlineStr<LEN> {
    fn borrow_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<const LEN: usize> AsRef<str> for InlineStr<LEN> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<const LEN: usize> AsMut<str> for InlineStr<LEN> {
    fn as_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<const LEN: usize> fmt::Display for InlineStr<LEN> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl<const LEN: usize> fmt::Debug for InlineStr<LEN> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl<const LEN: usize> Hash for InlineStr<LEN> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(self.as_str(), state)
    }
}

impl<'a, const LEN: usize> From<&'a InlineStr<LEN>> for &'a str {
    fn from(value: &'a InlineStr<LEN>) -> Self {
        value.as_str()
    }
}

impl<'a, const LEN: usize> TryFrom<&'a str> for &'a InlineStr<LEN> {
    type Error = FromStrError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        InlineStr::from_str(value)
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;
    use std::hash::{BuildHasher as _, RandomState};

    use super::InlineStr;

    #[test]
    fn from_utf8() {
        assert_eq!(
            InlineStr::from_utf8(*b"hello world").ok().as_deref(),
            Some("hello world"),
            "valid utf-8 must be ok to convert"
        );
        assert_eq!(
            InlineStr::from_utf8(*b"hello\xFFworld").ok().as_deref(),
            None,
            "invalid utf-8 should error"
        );
    }

    #[test]
    fn from_str() {
        assert!(
            InlineStr::<11>::from_str("hello world").is_ok(),
            "11 len str ok"
        );
        assert!(
            InlineStr::<11>::from_str("hello").is_err(),
            "5 != 11 len str ok"
        );
        assert!(
            InlineStr::<11>::from_str("hello, world!").is_err(),
            "13 != 11 len str ok"
        );
    }

    #[test]
    fn hash_eq() {
        let ref_str = "hello, world!";
        let inline = InlineStr::from_utf8(*b"hello, world!").expect("must be ok");

        let inline_borrow: &str = Borrow::borrow(&inline);

        let hash = RandomState::new();
        let ref_str_hash = hash.hash_one(ref_str);
        let inline_hash = hash.hash_one(inline);

        assert_eq!(ref_str, inline_borrow, "must be the same borrow value");
        assert_eq!(ref_str_hash, inline_hash, "must be the same hash");
    }
}
