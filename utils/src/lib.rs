pub mod text;
pub mod time;

/// Converts a slice to an array reference of size `N`.
/// This is essentially an unsafe version `<[T; N>::try_from`.
pub const unsafe fn as_with_size<'a, T, const N: usize>(slice: &'a [T]) -> &'a [T; N] {
    // SAFETY: The caller has to ensure that the length of `slice` is at least `N`.
    &*(slice.as_ptr() as *const [T; N])
}
