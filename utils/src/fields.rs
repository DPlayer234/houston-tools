//! Provides traits and macros to refer to a field of a struct.
#![deprecated = "use Fn traits instead"]

/// Represents access to a field of a struct.
///
/// Use the [`field`](crate::field) macro to obtain instances.
pub trait Field<S: ?Sized, F: ?Sized> {
    /// Gets a reference to the field.
    #[must_use]
    fn get<'r>(&self, obj: &'r S) -> &'r F;
}

/// Represents mutable access to a field of a struct.
///
/// Use the [`field_mut`](crate::field_mut) macro to obtain instances.
pub trait FieldMut<S: ?Sized, F: ?Sized>: Field<S, F> {
    /// Gets a mutable reference to the field.
    #[must_use]
    fn get_mut<'r>(&self, obj: &'r mut S) -> &'r mut F;
}

// Provide blanket implementations so any `&impl Field{Mut}<S, F>` is also `impl
// Field{Mut}<S, F>`.
impl<T, S: ?Sized, F: ?Sized> Field<S, F> for &T
where
    T: Field<S, F> + ?Sized,
{
    fn get<'r>(&self, obj: &'r S) -> &'r F {
        (**self).get(obj)
    }
}

impl<T, S: ?Sized, F: ?Sized> FieldMut<S, F> for &T
where
    T: FieldMut<S, F> + ?Sized,
{
    fn get_mut<'r>(&self, obj: &'r mut S) -> &'r mut F {
        (**self).get_mut(obj)
    }
}

/// Provides a [`Field`] implementation that uses lambdas.
///
/// This type isn't publicly available and hidden via `impl` in return position.
///
/// Note that the trait bounds for this type's generic parameters are only
/// present for its impl blocks.
//
// The reason this type even exists rather than providing implementations in the macros
// is that it allows inferring the field type. Sadly, there isn't a good way to also
// infer the input type.
#[derive(Debug, Clone, Copy)]
#[must_use]
struct LambdaField<Get, GetMut> {
    get: Get,
    get_mut: GetMut,
}

impl<S, F, Get, GetMut> Field<S, F> for LambdaField<Get, GetMut>
where
    S: ?Sized,
    F: ?Sized,
    Get: Fn(&S) -> &F,
{
    #[inline]
    fn get<'r>(&self, obj: &'r S) -> &'r F {
        (self.get)(obj)
    }
}

impl<S, F, Get, GetMut> FieldMut<S, F> for LambdaField<Get, GetMut>
where
    S: ?Sized,
    F: ?Sized,
    Get: Fn(&S) -> &F,
    GetMut: Fn(&mut S) -> &mut F,
{
    #[inline]
    fn get_mut<'r>(&self, obj: &'r mut S) -> &'r mut F {
        (self.get_mut)(obj)
    }
}

/// Creates a new [`Field`] from lambdas.
///
/// This isn't considered public API.
/// Instead, use the [`field`] macro to create values.
#[doc(hidden)]
pub const fn new_field<S, F, Get>(get: Get) -> impl Field<S, F>
where
    S: ?Sized,
    F: ?Sized,
    Get: Fn(&S) -> &F,
{
    LambdaField { get, get_mut: () }
}

/// Creates a new [`FieldMut`] from lambdas.
///
/// This isn't considered public API.
/// Instead, use the [`field_mut`] macro to create values.
#[doc(hidden)]
pub const fn new_field_mut<S, F, Get, GetMut>(get: Get, get_mut: GetMut) -> impl FieldMut<S, F>
where
    S: ?Sized,
    F: ?Sized,
    Get: Fn(&S) -> &F,
    GetMut: Fn(&mut S) -> &mut F,
{
    LambdaField { get, get_mut }
}

/// Gets a [`Field`] that refers to the provided info.
///
/// # Example
///
/// ```
/// use utils::fields::Field;
///
/// struct Chest {
///     treasure: usize
/// }
///
/// let field = utils::field!(Chest: treasure);
/// let chest = Chest {
///     treasure: 9999
/// };
///
/// assert_eq!(field.get(&chest), &9999);
/// ```
#[macro_export]
macro_rules! field {
    ($Type:ty : $($path:tt)*) => {{
        $crate::fields::new_field(
            |s: &$Type| &s.$($path)*,
        )
    }};
}

/// Gets a [`FieldMut`] that refers to the provided info.
///
/// # Example
///
/// ```
/// use utils::fields::FieldMut;
///
/// struct Chest {
///     treasure: usize
/// }
///
/// let field = utils::field_mut!(Chest: treasure);
/// let mut chest = Chest {
///     treasure: 0
/// };
///
/// *field.get_mut(&mut chest) = 9999;
/// assert_eq!(chest.treasure, 9999);
/// ```
#[macro_export]
macro_rules! field_mut {
    ($Type:ty : $($path:tt)*) => {{
        $crate::fields::new_field_mut(
            |s: &$Type| &s.$($path)*,
            |s: &mut $Type| &mut s.$($path)*,
        )
    }};
}
