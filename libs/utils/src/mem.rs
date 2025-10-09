//! Provides helper functions to work with blocks of memory.
//!
//! For example, this allows const-time conversion of slices into arrays via
//! [`as_sized`].

use std::slice;

/// Converts a slice to an array reference of size `N`.
/// This is a const-friendly alternative to `<&[T; N]>::try_from`.
///
/// Slices must have the right size.
///
/// # Panics
///
/// Panics if the slice is not exactly `N` long.
/// If you cannot guarantee this, use [`try_as_sized`].
///
/// # Examples
///
/// ```
/// let x: &[u8] = &[1, 2, 3, 4];
/// let y: &[u8; 4] = utils::mem::as_sized(x);
/// assert_eq!(x, y);
/// ```
#[must_use = "if you don't need the return value, just assert the length"]
pub const fn as_sized<T, const N: usize>(slice: &[T]) -> &[T; N] {
    try_as_sized(slice).expect("requested size should match slice length exactly")
}

/// Tries to convert a slice to an array reference of size `N`.
/// This is a const-friendly alternative to `<&[T; N]>::try_from`.
///
/// Returns [`None`] if the slice isn't exactly `N` long.
///
/// If you need a prefix or suffix of the slice instead, use
/// `[T]::first_chunk` or `[T]::last_chunk` instead.
///
/// # Examples
///
/// ```
/// let x: &[u8] = &[1, 2, 3, 4];
///
/// let exact = utils::mem::try_as_sized::<u8, 4>(x);
/// let small = utils::mem::try_as_sized::<u8, 2>(x);
/// let large = utils::mem::try_as_sized::<u8, 6>(x);
///
/// assert_eq!(exact, Some(&[1, 2, 3, 4]));
/// assert_eq!(small, None);
/// assert_eq!(large, None);
/// ```
#[must_use = "if you don't need the return value, just assert the length"]
pub const fn try_as_sized<T, const N: usize>(slice: &[T]) -> Option<&[T; N]> {
    if slice.len() == N {
        // SAFETY: The length has already been validated.
        Some(unsafe { &*slice.as_ptr().cast::<[T; N]>() })
    } else {
        None
    }
}

/// Transmutes a slice into raw bytes. Take note of endianness.
///
/// # Safety
///
/// Every bit of `slice` must be initialized. This isn't necessarily guaranteed
/// for every `T` since there may be unused bits within a given `T`.
///
/// If `T` is a primitive integer type, this is always safe.
///
/// # Example
///
/// ```
/// let slice: &[u16] = &[1, 2, 3];
/// let bytes = unsafe {
///     utils::mem::as_bytes(slice)
/// };
///
/// assert_eq!(bytes.len(), slice.len() * 2);
/// if cfg!(target_endian = "little") {
///     assert_eq!(bytes, &[1, 0, 2, 0, 3, 0]);
/// } else {
///     assert_eq!(bytes, &[0, 1, 0, 2, 0, 3]);
/// }
/// ```
#[must_use = "transmuting has no effect if you don't use the return value"]
pub const unsafe fn as_bytes<T>(slice: &[T]) -> &[u8] {
    let ptr = slice.as_ptr_range();

    // SAFETY: Both pointers are to the slice, so the offset must be valid.
    let byte_len = unsafe { ptr.end.byte_offset_from(ptr.start) };

    // SAFETY: Pointer is derived from a reference and byte length is known to be in
    // range and positive.
    unsafe { slice::from_raw_parts(ptr.start.cast(), byte_len.cast_unsigned()) }
}

/// Asserts that `T` is zero bytes in size or fails to compile.
pub const fn assert_zst<T>(value: T) -> T {
    const {
        assert!(size_of::<T>() == 0, "expected value to be zero-sized");
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_sized_correct_size() {
        let x: &[u8] = &[1, 2, 3, 4];
        let y: &[u8; 4] = as_sized(x);
        assert_eq!(x, y);
    }

    #[test]
    #[should_panic(expected = "requested size should match slice length exactly")]
    fn as_sized_too_small() {
        let x: &[u8] = &[1, 2, 3];
        let _y: &[u8; 4] = as_sized(x);
    }

    #[test]
    #[should_panic(expected = "requested size should match slice length exactly")]
    fn as_sized_too_large() {
        let x: &[u8] = &[1, 2, 3, 4, 5];
        let _y: &[u8; 4] = as_sized(x);
    }

    #[test]
    fn try_as_sized_correct_size() {
        let x: &[u8] = &[1, 2, 3, 4];
        let y: Option<&[u8; 4]> = try_as_sized(x);
        assert_eq!(y, Some(&[1, 2, 3, 4]));
    }

    #[test]
    fn try_as_sized_too_small() {
        let x: &[u8] = &[1, 2, 3];
        let y: Option<&[u8; 4]> = try_as_sized(x);
        assert_eq!(y, None);
    }

    #[test]
    fn try_as_sized_too_large() {
        let x: &[u8] = &[1, 2, 3, 4, 5];
        let y: Option<&[u8; 4]> = try_as_sized(x);
        assert_eq!(y, None);
    }
}
