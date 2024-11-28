#![doc(hidden)]
//! Needed for macro implementations. Not public API.

use std::ptr;

use super::InlineStr;

/// Given an ASCII or UTF-8 [`u8`] array representing a `SNAKE_CASE` string, converts it to title case (i.e. `Snake Case`).
///
/// This function is generally not useful and exists primarily to support the [`titlecase`](crate::titlecase) macro.
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
        offset = offset
            .checked_add(slices[slice_index].len())
            .expect("total length must not overflow");
        slice_index += 1;
    }

    offset
}

/// Provides a way to join several [`str`] slices.
///
/// This function is generally not useful and exists primarily to support the [`join`](crate::join) macro.
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
        assert!(offset + slice.len() <= N, "N was shorter than total input length");

        unsafe {
            // SAFETY: just checked that `slice` fits in `out`
            ptr::copy_nonoverlapping(slice.as_ptr(), out.as_mut_ptr().add(offset), slice.len());
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

#[cfg(test)]
mod tests {
    use std::hint::black_box;

    use super::join_str_const;

    #[test]
    #[should_panic = "N was shorter than total input length"]
    fn join_str_const_panics_too_short_n() {
        let slices = &["hello", "world"];
        black_box(join_str_const::<9>(slices));
    }

    #[test]
    #[should_panic = "total input length must be N"]
    fn join_str_const_panics_too_long_n() {
        let slices = &["hello", "world"];
        black_box(join_str_const::<11>(slices));
    }

    #[test]
    fn join_str_const_correct() {
        let value = const {
            let slices = &["hello", "world"];
            join_str_const::<10>(slices)
        };

        assert_eq!(value.as_str(), "helloworld");
    }
}
