//! A variety of utility types and functions for use across the crates in this repo.

// for benchmarks
#[cfg(test)]
use criterion as _;

pub mod fields;
pub mod fuzzy;
pub mod mem;
pub mod range;
pub mod str_as_data;
pub mod text;
pub mod time;

mod private;

/// Convenience method to calculate the hash of a value with the [`std::hash::DefaultHasher`].
#[must_use]
#[inline]
pub fn hash_default<T: std::hash::Hash + ?Sized>(value: &T) -> u64 {
    hash(value, std::hash::DefaultHasher::new())
}

/// Convenience method to feed a value to a hasher and then return its value.
#[must_use]
#[inline]
pub fn hash<T: std::hash::Hash + ?Sized, H: std::hash::Hasher>(value: &T, mut hasher: H) -> u64 {
    value.hash(&mut hasher);
    hasher.finish()
}

/// Joins multiple path segments into a [`PathBuf`](`std::path::PathBuf`).
///
/// An extension may be specified at the end. If specified, it will override the extension of the last segment.
///
/// This is equivalent to creating a [`PathBuf`](`std::path::PathBuf`) from the first segment and then repeatedly
/// calling `push`, then finishing with `set_extension` if an extension is specified.
///
/// # Example
///
/// ```
/// # use std::path::Path;
/// let path = utils::join_path!["C:\\", "Windows", "System32", "notepad"; "exe"];
/// # #[cfg(windows)]
/// assert_eq!(
///     &path,
///     Path::new(r#"C:\Windows\System32\notepad.exe"#)
/// )
/// ```
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

/// Provides overloads for an operator given a `[Op]Assign<&Rhs>` implementation.
///
/// In particular, this will provide both the by-value and by-ref overloads.
/// The `&Lhs` versions are only provided if the type is [`Copy`] and the macro invocation prefixes the type with `copy`.
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
    (copy $Lhs:ty, [$($TrAssign:tt)*] :: $assign:ident, [$($Tr:tt)*] :: $inline:ident) => {
        $crate::impl_op_via_assign!(copy $Lhs, Rhs=$Lhs, [$($TrAssign)*]::$assign, [$($Tr)*]::$inline);
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

#[cfg(test)]
mod test {
    use std::ops::{AddAssign, Add, SubAssign, Sub};

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
