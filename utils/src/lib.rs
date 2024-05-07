use std::fmt::Debug;
use std::marker::PhantomData;

pub mod prefix_map;
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

/// Represents a field of a struct. Provides methods to access the field.
#[must_use]
pub trait Field<S: ?Sized, F: ?Sized> {
    /// Gets a reference to the field.
    #[must_use]
    fn get<'r>(&self, obj: &'r S) -> &'r F;

    /// Gets a mutable reference to the field.
    #[must_use]
    fn get_mut<'r>(&self, obj: &'r mut S) -> &'r mut F;
}

/// Provides a [`Field`] implementation that uses lambdas.
#[must_use]
pub struct LambdaField<S: ?Sized, F: ?Sized, Get: Fn(&S) -> &F, GetMut: Fn(&mut S) -> &mut F> {
    get: Get,
    get_mut: GetMut,
    _phantom_s: PhantomData<S>,
    _phantom_f: PhantomData<F>,
}

impl<S: ?Sized, F: ?Sized, Get: Fn(&S) -> &F, GetMut: Fn(&mut S) -> &mut F> LambdaField<S, F, Get, GetMut> {
    pub const fn new(get: Get, get_mut: GetMut) -> Self {
        LambdaField {
            get, get_mut,
            _phantom_s: PhantomData,
            _phantom_f: PhantomData
        }
    }
}

impl<S: ?Sized, F: ?Sized, Get: Fn(&S) -> &F, GetMut: Fn(&mut S) -> &mut F> Field<S, F> for LambdaField<S, F, Get, GetMut> {
    fn get<'r>(&self, obj: &'r S) -> &'r F {
        (self.get)(obj)
    }

    fn get_mut<'r>(&self, obj: &'r mut S) -> &'r mut F {
        (self.get_mut)(obj)
    }
}

/// Gets a [`Field`] that refers to the provided info.
#[macro_export]
macro_rules! field {
    ($type:ty : $field:ident) => {{
        $crate::LambdaField::new(
            |s: &$type| &s.$field,
            |s: &mut $type| &mut s.$field
        )
    }};
}

#[macro_export]
macro_rules! define_simple_error {
    ($type:ident : $message:literal) => {
        #[derive(Debug, Clone)]
        pub struct $type;

        impl std::error::Error for $type {}

        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, $message)
            }
        }
    };
}

#[macro_export]
macro_rules! join_path {
    [$root:expr, $($parts:expr),* $(; $ext:expr)?] => {{
        let mut path = std::path::PathBuf::from($root);
        $(
            path.push($parts);
        )*
        $(
            path.set_extension($ext);
        )?
        path
    }};
}
