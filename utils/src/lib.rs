use std::fmt::Debug;

pub mod fields;
pub mod prefix_map;
pub mod range;
pub mod str_as_data;
pub mod text;
pub mod time;

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
#[must_use]
pub const fn as_with_size<'a, T, const N: usize>(slice: &'a [T]) -> &'a [T; N] {
    assert!(slice.len() >= N);
    unsafe {
        // SAFETY: The length has already been validated.
        &*(slice.as_ptr() as *const [T; N])
    }
}

/// Convenience method to calculate the hash of a value with the [`std::hash::DefaultHasher`].
#[inline]
pub fn hash_default<T: std::hash::Hash>(value: &T) -> u64 {
    hash(value, std::hash::DefaultHasher::new())
}

/// Convenience method to feed a value to a hasher and then return its value.
#[inline]
pub fn hash<T: std::hash::Hash, H: std::hash::Hasher>(value: &T, mut hasher: H) -> u64 {
    value.hash(&mut hasher);
    hasher.finish()
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
        if cfg!(debug_assertions) {
            drop(self.unwrap());
        }
    }
}

#[macro_export]
macro_rules! define_simple_error {
    ($type:ident : $message:literal) => {
        #[derive(Debug, Clone)]
        #[must_use]
        pub struct $type;

        impl ::std::error::Error for $type {}

        impl ::std::fmt::Display for $type {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, $message)
            }
        }
    };
}

#[macro_export]
macro_rules! join_path {
    [$root:expr, $($parts:expr),* $(; $ext:expr)?] => {{
        let mut path = ::std::path::PathBuf::from($root);
        $(
            path.push($parts);
        )*
        $(
            path.set_extension($ext);
        )?
        path
    }};
}
