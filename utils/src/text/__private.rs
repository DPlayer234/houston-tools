#![doc(hidden)]
//! Needed for macro implementations. Not public API.

use super::InlineStr;

/// Given an ASCII or UTF-8 [`u8`] array representing a `SNAKE_CASE` string, converts it to title case (i.e. `Snake Case`).
///
/// This function is generally not useful and exists primarily to support the [`titlecase`] macro.
#[must_use]
pub const fn to_titlecase_u8_array<const LEN: usize>(mut value: [u8; LEN]) -> [u8; LEN] {
    let mut is_start = true;

    let mut index = 0usize;
    while index < LEN {
        (value[index], is_start) = super::titlecase_impl::titlecase_transform(value[index], is_start);
        index += 1;
    }

    value
}

/// Counts the total length of all [`str`] slices.
///
/// # Panic
///
/// Panics if the total length of all slices overflows [`usize`].
#[must_use]
pub const fn count_str_const(slices: &[&str]) -> usize {
    let mut offset = 0usize;

    let mut slice_index = 0usize;
    while slice_index < slices.len() {
        offset = match offset.checked_add(slices[slice_index].len()) {
            Some(value) => value,
            None => panic!("total length overflows usize"),
        };
        slice_index += 1;
    }

    offset
}

/// Provides a way to join several [`str`] slices.
///
/// This function is generally not useful and exists primarily to support the [`join`] macro.
///
/// # Panic
///
/// Panics if `N` is not equal to the sum of the length of all slices.
#[must_use]
pub const fn join_str_const<const N: usize>(slices: &[&str]) -> InlineStr<N> {
    let mut out = [0u8; N];
    let mut offset = 0usize;

    let mut slice_index = 0usize;
    while slice_index < slices.len() {
        let slice = slices[slice_index].as_bytes();

        let mut index = 0usize;
        while index < slice.len() {
            out[offset + index] = slice[index];
            index += 1;
        }

        offset += slice.len();
        slice_index += 1;
    }

    assert!(offset == N, "total input length must be N");
    unsafe {
        // SAFETY: Only UTF-8 data was joined.
        InlineStr::from_utf8_unchecked(out)
    }
}
