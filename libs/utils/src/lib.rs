//! A variety of utility types and functions for use across the crates in this
//! repo.

// for benchmarks
#[cfg(test)]
use criterion as _;

pub mod fuzzy;
pub mod iter;
pub mod mem;
pub mod range;
pub mod str_as_data;
pub mod term;
pub mod text;

mod private;

#[expect(deprecated, reason = "to be removed in a future version")]
pub use private::hash::{hash, hash_default};

/// Joins multiple path segments into a [`PathBuf`].
///
/// An extension may be specified at the end. If specified, it will override the
/// extension of the last segment.
///
/// This is equivalent to creating a [`PathBuf`] from the first segment and then
/// repeatedly calling [`push`], then finishing with [`set_extension`] if an
/// extension is specified.
///
/// # Note
///
/// The use of [`set_extension`] may lead to some unexpected behavior:
///
/// - If the last component already has an extension, the extension will be
///   replaced.
/// - If the last component is `..`, no extension will be set.
///
/// See the docs for [`set_extension`] for further details.
///
/// # Example
///
/// ```
/// # use std::path::Path;
/// let path = utils::join_path!("C:\\", "Windows", "System32", "notepad"; "exe");
/// # #[cfg(windows)]
/// assert_eq!(
///     &path,
///     Path::new(r#"C:\Windows\System32\notepad.exe"#)
/// )
/// ```
///
/// [`PathBuf`]: std::path::PathBuf
/// [`push`]: std::path::PathBuf::push
/// [`set_extension`]: std::path::PathBuf::set_extension
#[macro_export]
macro_rules! join_path {
    ($root:expr $(,$parts:expr)* $(,)? $(; $ext:expr)?) => {{
        let mut path = ::std::path::PathBuf::from($root);
        $( path.push($parts); )*
        $( path.set_extension($ext); )?
        path
    }};
}

/// Provides overloads for an operator given a `[Op]Assign<&Rhs>`
/// implementation.
///
/// In particular, this will provide both the by-value and by-ref overloads.
/// The `&Lhs` versions are only provided if the type is [`Copy`] and the macro
/// invocation prefixes the type with `copy`.
///
/// Furthermore, it also provides the `[Op]Assign<Rhs>` (Rhs by-value) overload.
///
/// # Examples
///
/// ```
/// use std::ops::{BitOr, BitOrAssign};
///
/// #[derive(Clone, Copy, PartialEq)]
/// struct Flags(i32);
///
/// impl BitOrAssign<&Self> for Flags {
///     fn bitor_assign(&mut self, rhs: &Self) {
///         self.0 |= rhs.0;
///     }
/// }
///
/// utils::impl_op_via_assign!(copy Flags, [BitOrAssign]::bitor_assign, [BitOr]::bitor);
///
/// assert!(Flags(0b01) | Flags(0b10) == Flags(0b11));
/// ```
///
/// You can also specify a different Rhs type.
///
/// ```
/// use std::ops::{BitOr, BitOrAssign};
///
/// #[derive(PartialEq)]
/// struct RawFlags(i32);
///
/// impl BitOrAssign<&i32> for RawFlags {
///     fn bitor_assign(&mut self, rhs: &i32) {
///         self.0 |= rhs;
///     }
/// }
///
/// utils::impl_op_via_assign!(RawFlags, Rhs=i32, [BitOrAssign]::bitor_assign, [BitOr]::bitor);
///
/// assert!(RawFlags(0b01) | 0b10 == RawFlags(0b11));
/// ```
#[macro_export]
macro_rules! impl_op_via_assign {
    ($Lhs:ty, [$($TrAssign:tt)*] :: $assign:ident, [$($Tr:tt)*] :: $inline:ident) => {
        $crate::impl_op_via_assign!($Lhs, Rhs=$Lhs, [$($TrAssign)*]::$assign, [$($Tr)*]::$inline);
    };
    ($Lhs:ty, Rhs=$Rhs:ty, [$($TrAssign:tt)*] :: $assign:ident, [$($Tr:tt)*] :: $inline:ident) => {
        impl $($Tr)*<$Rhs> for $Lhs {
            type Output = $Lhs;

            #[inline]
            fn $inline(mut self, rhs: $Rhs) -> $Lhs {
                $($TrAssign)*::$assign(&mut self, &rhs);
                self
            }
        }

        impl $($Tr)*<&$Rhs> for $Lhs {
            type Output = $Lhs;

            #[inline]
            fn $inline(mut self, rhs: &$Rhs) -> $Lhs {
                $($TrAssign)*::$assign(&mut self, rhs);
                self
            }
        }

        impl $($TrAssign)*<$Rhs> for $Lhs {
            #[inline]
            fn $assign(&mut self, rhs: $Rhs) {
                $($TrAssign)*::$assign(self, &rhs);
            }
        }
    };
    (copy $Lhs:ty, [$($TrAssign:tt)*] :: $assign:ident, [$($Tr:tt)*] :: $inline:ident) => {
        $crate::impl_op_via_assign!(copy $Lhs, Rhs=$Lhs, [$($TrAssign)*]::$assign, [$($Tr)*]::$inline);
    };
    (copy $Lhs:ty, Rhs=$Rhs:ty, [$($TrAssign:tt)*] :: $assign:ident, [$($Tr:tt)*] :: $inline:ident) => {
        $crate::impl_op_via_assign!($Lhs, Rhs=$Rhs, [$($TrAssign)*]::$assign, [$($Tr)*]::$inline);

        impl $($Tr)*<$Rhs> for &$Lhs where $Lhs: ::std::marker::Copy {
            type Output = $Lhs;

            #[inline]
            fn $inline(self, rhs: $Rhs) -> $Lhs {
                $($Tr)*::$inline(*self, &rhs)
            }
        }

        impl $($Tr)*<&$Rhs> for &$Lhs where $Lhs: ::std::marker::Copy {
            type Output = $Lhs;

            #[inline]
            fn $inline(self, rhs: &$Rhs) -> $Lhs {
                $($Tr)*::$inline(*self, rhs)
            }
        }
    };
}

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
        $crate::impl_debug!(@struct ($pref.field(stringify!($field), &$field)) $($($rest)*)?)
    };
    // recursively add another field, but:
    // tt instead of ident for $field so it can be used with tuple structs
    // $as is the renamed local but is otherwise meaningless
    (@struct ($pref:expr) $field:tt: $as:ident $(, $($rest:tt)*)?) => {
        $crate::impl_debug!(@struct ($pref.field(stringify!($field), &$as)) $($($rest)*)?)
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
            ($($out,)* $f.write_str(stringify!($Var)))
            $($($body)*)?
        )
    };
    // otherwise delegate to the regular struct handling
    (@enum $self:expr, $f:expr, ($($pat:pat),*) ($($out:expr),*) $Var:ident $var_body:tt $(, $($body:tt)*)?) => {
        $crate::impl_debug!(@enum $self, $f,
            ($($pat,)* Self::$Var $var_body)
            ($($out,)* $crate::impl_debug!(@bodystart $f, stringify!($Var), $var_body))
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

#[cfg(test)]
mod test {
    use std::ops::{Add, AddAssign, Sub, SubAssign};

    #[derive(Clone, Copy, PartialEq, Eq)]
    struct Num(i32);

    impl AddAssign<&Self> for Num {
        fn add_assign(&mut self, rhs: &Self) {
            self.0 += rhs.0;
        }
    }

    impl SubAssign<&Self> for Num {
        fn sub_assign(&mut self, rhs: &Self) {
            self.0 -= rhs.0;
        }
    }

    impl AddAssign<&i32> for Num {
        fn add_assign(&mut self, rhs: &i32) {
            self.0 += rhs;
        }
    }

    impl SubAssign<&i32> for Num {
        fn sub_assign(&mut self, rhs: &i32) {
            self.0 -= rhs;
        }
    }

    super::impl_op_via_assign!(copy Num, [AddAssign]::add_assign, [Add]::add);
    super::impl_op_via_assign!(copy Num, [SubAssign]::sub_assign, [Sub]::sub);
    super::impl_op_via_assign!(copy Num, Rhs=i32, [AddAssign]::add_assign, [Add]::add);
    super::impl_op_via_assign!(copy Num, Rhs=i32, [SubAssign]::sub_assign, [Sub]::sub);

    #[test]
    fn add_correct() {
        fn add<L: Add<R>, R>(l: L, r: R) -> L::Output {
            l + r
        }

        fn add_assign<L: AddAssign<R>, R>(mut l: L, r: R) -> L {
            l += r;
            l
        }

        assert!(add(&Num(1), &Num(2)) == Num(3));
        assert!(add(&Num(2), Num(4)) == Num(6));
        assert!(add(Num(3), &Num(6)) == Num(9));
        assert!(add(Num(4), Num(8)) == Num(12));

        assert!(add_assign(Num(3), &Num(6)) == Num(9));
        assert!(add_assign(Num(4), Num(8)) == Num(12));

        assert!(add(&Num(1), &2) == Num(3));
        assert!(add(&Num(2), 4) == Num(6));
        assert!(add(Num(3), &6) == Num(9));
        assert!(add(Num(4), 8) == Num(12));

        assert!(add_assign(Num(3), &6) == Num(9));
        assert!(add_assign(Num(4), 8) == Num(12));
    }

    #[test]
    fn sub_correct() {
        fn sub<L: Sub<R>, R>(l: L, r: R) -> L::Output {
            l - r
        }

        fn sub_assign<L: SubAssign<R>, R>(mut l: L, r: R) -> L {
            l -= r;
            l
        }

        assert!(sub(&Num(2), &Num(1)) == Num(1));
        assert!(sub(Num(4), &Num(2)) == Num(2));
        assert!(sub(&Num(6), Num(3)) == Num(3));
        assert!(sub(Num(8), Num(4)) == Num(4));

        assert!(sub_assign(Num(6), &Num(3)) == Num(3));
        assert!(sub_assign(Num(8), Num(4)) == Num(4));

        assert!(sub(&Num(2), &1) == Num(1));
        assert!(sub(Num(4), &2) == Num(2));
        assert!(sub(&Num(6), 3) == Num(3));
        assert!(sub(Num(8), 4) == Num(4));

        assert!(sub_assign(Num(6), &3) == Num(3));
        assert!(sub_assign(Num(8), 4) == Num(4));
    }
}
