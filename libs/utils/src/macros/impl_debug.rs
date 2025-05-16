/// Provides a [`Debug`](std::fmt::Debug) implementation, similar to the
/// derive-based version, only including listed fields and with custom generic
/// bounds.
///
/// # Examples
///
/// Fields can be omitted. `..` must be included in case of missing fields:
///
/// ```no_run
/// struct Login {
///     username: String,
///     password: String,
/// }
///
/// // no password in the Debug output
/// utils::impl_debug!(struct Login: { username, .. });
/// ```
///
/// You can also specify generics with custom bounds.
/// All the fields must still impl Debug in all allowed cases.
/// The generics for the impl must be specified via `for[..]`.
///
/// ```no_run
/// # use std::marker::PhantomData;
/// struct Raw<T> {
///     ptr: *const (),
///     _marker: PhantomData<T>,
/// }
///
/// // impl Debug even for T that don't impl Debug
/// utils::impl_debug!(for[T] struct Raw<T>: { ptr, .. });
/// ```
///
/// You will need to repeat bounds on the type:
///
/// ```no_run
/// # use std::fmt::Debug;
/// struct Sender<T: Send> {
///     buf: Vec<T>,
/// }
///
/// utils::impl_debug!(for[T: Send + Debug] struct Sender<T>: { buf });
/// ```
///
/// Enums are also supported. You will need to list every variant:
///
/// ```no_run
/// enum Status {
///     Int(i32),
///     String { str: String },
///     Unknown,
/// }
///
/// utils::impl_debug!(enum Status: {
///     Int(i),
///     String { str },
///     Unknown,
/// });
/// ```
///
/// When used with tuple-structs, you will need to provide variable names for
/// the fields for the macro to use internally. Note that the original field
/// names will be printed regardless.
///
/// ```no_run
/// struct Block(u64, u64, u64, String);
///
/// utils::impl_debug!(struct Block: { 0: _0, 1: _1, 2: _2, .. });
/// ```
///
/// You can also use tuple or unit syntax, but you won't be able to omit fields
/// in the middle:
///
/// ```no_run
/// struct Chunk(u64, u64, String);
///
/// utils::impl_debug!(struct Chunk: (_0, _1, ..));
/// ```
#[macro_export]
macro_rules! impl_debug {
    // handling for "struct bodies", i.e. differentiating between struct/tuple/unit syntax
    (@bodystart $f:expr, $name:expr, { $($body:tt)* }) => {
        $crate::impl_debug!(@struct ($f.debug_struct($name)) $($body)*)
    };
    (@bodystart $f:expr, $name:expr, ( $($body:tt)* )) => {
        $crate::impl_debug!(@tuple ($f.debug_tuple($name)) $($body)*)
    };
    (@bodystart $f:expr, $name:expr,) => {
        $f.write_str($name)
    };

    // omit remaining fields
    (@struct ($pref:expr) ..) => {
        $pref.finish_non_exhaustive()
    };
    // fully exhausted input
    (@struct ($pref:expr) $(,)?) => {
        $pref.finish()
    };
    // recursively add another field
    (@struct ($pref:expr) $field:ident $(, $($rest:tt)*)?) => {
        $crate::impl_debug!(@struct ($pref.field(::std::stringify!($field), &$field)) $($($rest)*)?)
    };
    // recursively add another field, but:
    // tt instead of ident for $field so it can be used with tuple structs
    // $as is the renamed local but is otherwise meaningless
    (@struct ($pref:expr) $field:tt: $as:ident $(, $($rest:tt)*)?) => {
        $crate::impl_debug!(@struct ($pref.field(::std::stringify!($field), &$as)) $($($rest)*)?)
    };

    // omit remaining fields
    (@tuple ($pref:expr) ..) => {
        $pref.finish_non_exhaustive()
    };
    // fully exhausted input
    (@tuple ($pref:expr) $(,)?) => {
        $pref.finish()
    };
    // recursively add another field
    (@tuple ($pref:expr) $field:ident $(, $($rest:tt)*)?) => {
        $crate::impl_debug!(@tuple ($pref.field(&$field)) $($($rest)*)?)
    };

    // accumulator/tt muncher to handle inserting macros in pattern position
    // fully exhausted input
    (@enum $self:expr, $f:expr, ($($pat:pat),*) ($($out:expr),*) $(,)?) => {
        match $self { $( $pat => $out ),* }
    };
    // special-case unit since we need to handle the "comma next" case differently
    (@enum $self:expr, $f:expr, ($($pat:pat),*) ($($out:expr),*) $Var:ident $(, $($body:tt)*)?) => {
        $crate::impl_debug!(@enum $self, $f,
            ($($pat,)* Self::$Var)
            ($($out,)* $f.write_str(::std::stringify!($Var)))
            $($($body)*)?
        )
    };
    // otherwise delegate to the regular struct handling
    (@enum $self:expr, $f:expr, ($($pat:pat),*) ($($out:expr),*) $Var:ident $var_body:tt $(, $($body:tt)*)?) => {
        $crate::impl_debug!(@enum $self, $f,
            ($($pat,)* Self::$Var $var_body)
            ($($out,)* $crate::impl_debug!(@bodystart $f, ::std::stringify!($Var), $var_body))
            $($($body)*)?
        )
    };

    // the colon after the type is needed since $ty can't be followed by $tt or `(`
    // applied to enums for consistency, even though it isn't needed there
    // we can't match the type with $tt because that excludes the generics
    ($(for [$($bound:tt)*])? struct $Ty:ty: $($body:tt)?) => {
        impl $(<$($bound)*>)? ::std::fmt::Debug for $Ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    Self $($body)? => $crate::impl_debug!(@bodystart f, stringify!($Ty), $($body)?)
                }
            }
        }
    };

    ($(for [$($bound:tt)*])? enum $Ty:ty: { $($body:tt)* }) => {
        impl $(<$($bound)*>)? ::std::fmt::Debug for $Ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                $crate::impl_debug!(@enum self, f, () () $($body)*)
            }
        }
    };
}
