//! Provides helper functions to work with blocks of memory.
//!
//! For example, this allows const-time conversion of slices into arrays via [`as_sized`].

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
#[inline]
#[must_use = "if you don't need the return value, just assert the length"]
pub const fn as_sized<T, const N: usize>(slice: &[T]) -> &[T; N] {
    match try_as_sized(slice) {
        Some(slice) => slice,
        None => panic!("requested size incorrect"),
    }
}

/// Tries to convert a slice to an array reference of size `N`.
/// This is a const-friendly alternative to `<&[T; N]>::try_from`.
///
/// Returns [`None`] if the slice isn't exactly `N` long.
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
#[inline]
pub const fn try_as_sized<T, const N: usize>(slice: &[T]) -> Option<&[T; N]> {
    if slice.len() == N {
        Some(unsafe {
            // SAFETY: The length has already been validated.
            &*slice.as_ptr().cast::<[T; N]>()
        })
    } else {
        None
    }
}

/// Transmutes a slice of some type into one of another.
///
/// The length of the new slice is adjusted to cover the same memory region without
/// going out of bounds of the original slice.
///
/// # Safety
///
/// The start of `slice` must have a supported alignment for `Dst`.
/// This is required when even the slice is empty.
///
/// The start of the new slice will be the same pointer as the original slice.
///
/// The length will chosen as such:
/// - If either of `Src` or `Dst`, but not both, are zero-sized types, the new slice will be empty.
/// - If both are zero-sized types, the new slice will have the same length as the original.
/// - Otherwise, the length will be `size_of::<Src>() * len / size_of::<Dst>()`, truncating away the
///   end section that doesn't fit another `Dst`.
///
/// The memory of `slice` must be valid for every `Dst` produced.
///
/// There is no guarantee this operation is reversible, i.e. this may fail:
/// ```no_run
/// # use utils::mem::transmute_slice;
/// unsafe {
///     let bytes: &[u8] = &[1, 2, 3];
///     let shorts: &[u16] = transmute_slice(bytes);
///     let not_bytes: &[u8] = transmute_slice(shorts);
///     assert_eq!(bytes, not_bytes);
/// }
/// ```
#[inline]
#[must_use = "transmuting has no effect if you don't use the return value"]
#[allow(clippy::cast_sign_loss)]
pub const unsafe fn transmute_slice<Src, Dst>(slice: &[Src]) -> &[Dst] {
    let ptr = slice.as_ptr_range();

    // `<*const T>::is_aligned` is not yet const-stable.
    // Uncomment the following line when it is:
    // debug_assert!(ptr.start.cast::<Dst>().is_aligned());

    // SAFETY: Both pointers are to the slice, so the offset must be valid.
    let byte_len = unsafe { ptr.end.byte_offset_from(ptr.start) };
    debug_assert!(byte_len >= 0, "sanity: end >= start, so byte_len must be positive");

    let src_size = size_of::<Src>();
    let dst_size = size_of::<Dst>();

    unsafe {
        let dst_len = match (src_size, dst_size) {
            (0, 0) => slice.len(),
            (_, 0) | (0, _) => 0,
            _ => (byte_len as usize) / dst_size,
        };

        std::slice::from_raw_parts(ptr.start.cast(), dst_len)
    }
}

/// Transmutes a slice into raw bytes.
///
/// This equivalent to [`transmute_slice`] with a `Dst` of [`u8`].
///
/// # Safety
///
/// Every bit of `slice` must be initialized.
/// This isn't necessarily guaranteed for every `T`
/// since there may be unused bits within a given `T`.
///
/// # Example
///
/// ```
/// # use utils::mem::as_bytes;
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
#[inline]
#[must_use = "transmuting has no effect if you don't use the return value"]
pub const unsafe fn as_bytes<T>(slice: &[T]) -> &[u8] {
    unsafe {
        transmute_slice(slice)
    }
}
