//! Provides helper functions to work with blocks of memory.

use std::slice;

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
    let byte_len = size_of_val(slice);
    let ptr = slice.as_ptr().cast();

    // SAFETY: pointer is derived from a reference and length is based on
    // `size_of_val` so it must match the original object and is in range
    unsafe { slice::from_raw_parts(ptr, byte_len) }
}

/// This function asserts that `T` is a zero-sized type and returns the input.
///
/// If the input is not zero-sized, fails to compile.
#[inline(always)]
pub const fn assert_zst<T>(value: T) -> T {
    const {
        assert!(size_of::<T>() == 0, "expected value to be zero-sized");
    }
    value
}
