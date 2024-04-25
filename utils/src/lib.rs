use std::fmt::Debug;

pub mod discord_fmt;
pub mod text;
pub mod time;
pub mod prefix_map;

/// Converts a slice to an array reference of size `N`.
/// This is a const-friendly alternative to `<&[T; N]>::try_from`.
/// 
/// # Panics
/// 
/// Panics if the slice is shorter than `N`. Longer slices are allowed.
/// 
/// # Examples
/// 
/// ```
/// let x: &[u8] = &[1, 2, 3, 4];
/// let y: &[u8; 4] = utils::as_with_size(x);
/// assert_eq!(x, y);
/// ```
pub const fn as_with_size<'a, T, const N: usize>(slice: &'a [T]) -> &'a [T; N] {
    assert!(slice.len() >= N);
    unsafe {
        // SAFETY: The length has already been validated.
        &*(slice.as_ptr() as *const [T; N])
    }
}

/// Trait that allows discarding values.
pub trait Discard {
    /// Consumes and discards the value.
    /// May panic if debug assertions are enabled.
    fn discard(self);
}

impl<T, E: Debug> Discard for Result<T, E> {
    /// Consumes and discards the value.
    /// If debug assertions are enabled, panics if it holds an error.
    fn discard(self) {
        #[cfg(debug_assertions)]
        drop(self.unwrap());

        #[cfg(not(debug_assertions))]
        drop(self);
    }
}
