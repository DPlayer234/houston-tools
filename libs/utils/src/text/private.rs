#![doc(hidden)]
//! Needed for macro implementations. Not public API.

use super::InlineStr;
pub use super::titlecase_impl::to_titlecase_u8;
use crate::iter::ConstIter;

/// Counts the total length of all [`str`] slices.
///
/// This function is generally not useful and exists primarily to support the
/// [`join`](crate::join) macro.
///
/// # Panic
///
/// Panics if the total length of all slices overflows [`usize`].
#[must_use]
pub const fn count_str_const(slices: &[&str]) -> usize {
    let mut offset = 0usize;

    let mut iter = ConstIter::new(slices);
    while let Some(slice) = iter.next() {
        offset = offset
            .checked_add(slice.len())
            .expect("total length must not overflow");
    }

    offset
}

/// Provides a way to join several [`str`] slices.
///
/// This function is generally not useful and exists primarily to support the
/// [`join`](crate::join) macro.
///
/// # Panic
///
/// Panics if `N` is not equal to the sum of the length of all slices.
#[must_use]
pub const fn join_str_const<const N: usize>(slices: &[&str]) -> InlineStr<N> {
    let mut out = [0u8; N];
    join_str_const_into(slices, &mut out);

    // SAFETY: `out` contains only valid UTF-8.
    unsafe { InlineStr::from_utf8_unchecked(out) }
}

/// Non-generic helper for [`join_str_const`].
///
/// When this function does not panic, it is guaranteed that `out` has been
/// fully written and only contains valid UTF-8.
///
/// # Panic
///
/// Panics if `out.len()` is not equal to the sum of the length of all slices.
const fn join_str_const_into(slices: &[&str], mut out: &mut [u8]) {
    let mut iter = ConstIter::new(slices);
    while let Some(&slice) = iter.next() {
        let Some((part, rest)) = out.split_at_mut_checked(slice.len()) else {
            panic!("N was shorter than total input length");
        };

        part.copy_from_slice(slice.as_bytes());
        out = rest;
    }

    assert!(out.is_empty(), "total input length must be N");
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
