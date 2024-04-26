use std::{fmt::Debug, marker::PhantomData};

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

pub trait Field<S: ?Sized, F: ?Sized> {
    fn get<'r>(&self, obj: &'r S) -> &'r F;
    fn get_mut<'r>(&self, obj: &'r mut S) -> &'r mut F;
}

pub struct LambdaField<S: ?Sized, F: ?Sized, Get: Fn(&S) -> &F, GetMut: Fn(&mut S) -> &mut F> {
    pub get: Get,
    pub get_mut: GetMut,
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

#[macro_export]
macro_rules! field {
    ($type:ty : $field:ident) => {{
        $crate::LambdaField::new(
            |s: &$type| &s.$field,
            |s: &mut $type| &mut s.$field
        )
    }};
}
