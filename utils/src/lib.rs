use std::fmt::Debug;

pub mod text;
pub mod time;

/// Converts a slice to an array reference of size `N`.
/// This is essentially an unsafe version `<[T; N]>::try_from`.
pub const unsafe fn as_with_size<'a, T, const N: usize>(slice: &'a [T]) -> &'a [T; N] {
    // SAFETY: The caller has to ensure that the length of `slice` is at least `N`.
    &*(slice.as_ptr() as *const [T; N])
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
