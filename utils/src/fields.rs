/// Represents access to a field of a struct.
///
/// Use the [`field`] macro to obtain instances.
pub trait Field<S: ?Sized, F: ?Sized> {
    /// Gets a reference to the field.
    #[must_use]
    fn get<'r>(&self, obj: &'r S) -> &'r F;
}

/// Represents mutable access to a field of a struct.
///
/// Use the [`field_mut`] macro to obtain instances.
pub trait FieldMut<S: ?Sized, F: ?Sized>: Field<S, F> {
    /// Gets a mutable reference to the field.
    #[must_use]
    fn get_mut<'r>(&self, obj: &'r mut S) -> &'r mut F;
}

/// Provides a [`Field`] implementation that uses lambdas.
///
/// It is discouraged to use this struct directly.
/// Instead, use the [`Field`] trait and [`field`] macro or
/// the [`FieldMut`] trait and [`field_mut`] macro.
///
/// Note that the trait bounds for this type's generic parameters are only present for its impl blocks.
//
// The reason this type even exists rather than providing implementations in the macros
// is that it allows inferring the field type. Sadly, there isn't a good way to also
// infer the input type.
#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct LambdaField<Get, GetMut = ()> {
    get: Get,
    get_mut: GetMut,
}

impl<Get> LambdaField<Get> {
    /// Creates a new [`LambdaField`] from lambdas.
    ///
    /// Instead, use the [`field`] macro to create values.
    pub const fn new<S: ?Sized, F: ?Sized>(get: Get) -> Self
    where Get: Fn(&S) -> &F {
        LambdaField {
            get,
            get_mut: (),
        }
    }
}

impl<Get, GetMut> LambdaField<Get, GetMut> {
    /// Creates a new [`LambdaField`] from lambdas.
    ///
    /// Instead, use the [`field_mut`] macro to create values.
    pub const fn new_mut<S: ?Sized, F: ?Sized>(get: Get, get_mut: GetMut) -> Self
    where Get: Fn(&S) -> &F, GetMut: Fn(&mut S) -> &mut F {
        LambdaField {
            get,
            get_mut,
        }
    }
}

impl<S: ?Sized, F: ?Sized, Get: Fn(&S) -> &F, GetMut> Field<S, F> for LambdaField<Get, GetMut> {
    #[inline]
    fn get<'r>(&self, obj: &'r S) -> &'r F {
        (self.get)(obj)
    }
}

impl<S: ?Sized, F: ?Sized, Get: Fn(&S) -> &F, GetMut: Fn(&mut S) -> &mut F> FieldMut<S, F> for LambdaField<Get, GetMut> {
    #[inline]
    fn get_mut<'r>(&self, obj: &'r mut S) -> &'r mut F {
        (self.get_mut)(obj)
    }
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
        $crate::fields::LambdaField::new(
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
        $crate::fields::LambdaField::new_mut(
            |s: &$Type| &s.$($path)*,
            |s: &mut $Type| &mut s.$($path)*,
        )
    }};
}
