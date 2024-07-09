/// Converts a slice to an array reference of size `N`.
/// This is a const-friendly alternative to `<&[T; N]>::try_from`.
///
/// # Panics
///
/// Panics if the slice is shorter than `N`. Longer slices are allowed but truncated.
///
/// # Examples
///
/// ```
/// let x: &[u8] = &[1, 2, 3, 4];
/// let y: &[u8; 4] = utils::mem::with_size(x);
/// assert_eq!(x, y);
/// ```
#[must_use]
pub const fn with_size<'a, T, const N: usize>(slice: &'a [T]) -> &'a [T; N] {
    assert!(slice.len() >= N);
    unsafe {
        // SAFETY: The length has already been validated.
        &*(slice.as_ptr() as *const [T; N])
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
/// The memory of `slice` must be valid for every `Dst` produced.
///
/// The size, in bytes, of `slice` does not have to be a multiple of the size of `Dst`,
/// however the length will be rounded down and the unused part will be truncated.
///
/// In that sense, the start of the new slice is guaranteed to be the same, but the end isn't.
///
/// If `Dst` is a zero-sized type, the new slice will have the same length as the original.
/// If `Src` is a zero-sized type and `Dst` isn't, the new slice will be empty.
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
///
/// # Example
///
///
#[must_use]
pub const unsafe fn transmute_slice<Src, Dst>(slice: &[Src]) -> &[Dst] {
    let ptr = slice.as_ptr_range();

    // `<*const T>::is_aligned` is not yet const-stable.
    // Uncomment the following line when it is:
    // debug_assert!(ptr.start.cast::<Dst>().is_aligned());

    let byte_len = ptr.end.byte_offset_from(ptr.start);
    let dst_size = std::mem::size_of::<Dst>();

    debug_assert!(byte_len >= 0);

    unsafe {
        let dst_len = if dst_size == 0 {
            slice.len()
        } else {
            (byte_len as usize) / dst_size
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
#[must_use]
pub const unsafe fn as_bytes<T>(slice: &[T]) -> &[u8] {
    unsafe {
        transmute_slice(slice)
    }
}
