use std::fmt::Debug;

pub mod discord_fmt;
pub mod text;
pub mod time;

/// Converts a slice to an array reference of size `N`.
/// This is a const-friendly alternative to `<&[T; N]>::try_from`.
/// 
/// If the slice is shorter than `N`, this method will panic.
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
