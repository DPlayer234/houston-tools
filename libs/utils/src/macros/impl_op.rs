/// Provides overloads for an operator given a `[Op]Assign<&Rhs>`
/// implementation.
///
/// In particular, this will provide both the by-value and by-ref overloads.
/// The `&Lhs` versions are only provided if the type is [`Copy`].
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
/// utils::impl_op_via_assign!(Flags, [BitOrAssign]::bitor_assign, [BitOr]::bitor);
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
    (for [$($bound:tt)*] $Lhs:ty, [$($TrAssign:tt)*] :: $assign:ident, [$($Tr:tt)*] :: $inline:ident) => {
        $crate::impl_op_via_assign!(for[$($bound)*] $Lhs, Rhs=$Lhs, [$($TrAssign)*]::$assign, [$($Tr)*]::$inline);
    };
    (for [$($bound:tt)*] $Lhs:ty, Rhs=$Rhs:ty, [$($TrAssign:tt)*] :: $assign:ident, [$($Tr:tt)*] :: $inline:ident) => {
        impl <$($bound)*> $($Tr)*<$Rhs> for $Lhs {
            type Output = $Lhs;

            #[inline]
            fn $inline(mut self, rhs: $Rhs) -> $Lhs {
                $($TrAssign)*::$assign(&mut self, &rhs);
                self
            }
        }

        impl <$($bound)*> $($Tr)*<&$Rhs> for $Lhs {
            type Output = $Lhs;

            #[inline]
            fn $inline(mut self, rhs: &$Rhs) -> $Lhs {
                $($TrAssign)*::$assign(&mut self, rhs);
                self
            }
        }

        impl <$($bound)*> $($TrAssign)*<$Rhs> for $Lhs {
            #[inline]
            fn $assign(&mut self, rhs: $Rhs) {
                $($TrAssign)*::$assign(self, &rhs);
            }
        }

        impl <$($bound)*> $($Tr)*<$Rhs> for &$Lhs where for<'__dummy> $Lhs: ::std::marker::Copy {
            type Output = $Lhs;

            #[inline]
            fn $inline(self, rhs: $Rhs) -> $Lhs {
                $($Tr)*::$inline(*self, &rhs)
            }
        }

        impl <$($bound)*> $($Tr)*<&$Rhs> for &$Lhs where for<'__dummy> $Lhs: ::std::marker::Copy {
            type Output = $Lhs;

            #[inline]
            fn $inline(self, rhs: &$Rhs) -> $Lhs {
                $($Tr)*::$inline(*self, rhs)
            }
        }
    };
    ($Lhs:ty, [$($TrAssign:tt)*] :: $assign:ident, [$($Tr:tt)*] :: $inline:ident) => {
        $crate::impl_op_via_assign!(for[] $Lhs, [$($TrAssign)*]::$assign, [$($Tr)*]::$inline);
    };
    ($Lhs:ty, Rhs=$Rhs:ty, [$($TrAssign:tt)*] :: $assign:ident, [$($Tr:tt)*] :: $inline:ident) => {
        $crate::impl_op_via_assign!(for[] $Lhs, Rhs=$Rhs, [$($TrAssign)*]::$assign, [$($Tr)*]::$inline);
    };
    (copy $Lhs:ty, [$($TrAssign:tt)*] :: $assign:ident, [$($Tr:tt)*] :: $inline:ident) => {
        $crate::impl_op_via_assign!(copy $Lhs, Rhs=$Lhs, [$($TrAssign)*]::$assign, [$($Tr)*]::$inline);
    };
    (copy $Lhs:ty, Rhs=$Rhs:ty, [$($TrAssign:tt)*] :: $assign:ident, [$($Tr:tt)*] :: $inline:ident) => {
        const _: () = {
            #[deprecated = "Copy is now detected automatically and conditionally, don't specify `copy` in the macro"]
            const COPY: () = ();
            COPY
        };

        $crate::impl_op_via_assign!($Lhs, Rhs=$Rhs, [$($TrAssign)*]::$assign, [$($Tr)*]::$inline);
    };
}

#[cfg(test)]
mod tests {
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

    crate::impl_op_via_assign!(Num, [AddAssign]::add_assign, [Add]::add);
    crate::impl_op_via_assign!(Num, [SubAssign]::sub_assign, [Sub]::sub);
    crate::impl_op_via_assign!(Num, Rhs=i32, [AddAssign]::add_assign, [Add]::add);
    crate::impl_op_via_assign!(Num, Rhs=i32, [SubAssign]::sub_assign, [Sub]::sub);

    #[derive(Clone, Copy, PartialEq, Eq)]
    struct NumGeneric<T>(T);

    impl<T: for<'a> AddAssign<&'a T>> AddAssign<&Self> for NumGeneric<T> {
        fn add_assign(&mut self, rhs: &Self) {
            self.0 += &rhs.0;
        }
    }

    crate::impl_op_via_assign!(for[T: for<'a> AddAssign<&'a T>] NumGeneric<T>, [AddAssign]::add_assign, [Add]::add);

    fn add<L: Add<R>, R>(l: L, r: R) -> L::Output {
        l + r
    }

    fn add_assign<L: AddAssign<R>, R>(mut l: L, r: R) -> L {
        l += r;
        l
    }

    fn sub<L: Sub<R>, R>(l: L, r: R) -> L::Output {
        l - r
    }

    fn sub_assign<L: SubAssign<R>, R>(mut l: L, r: R) -> L {
        l -= r;
        l
    }

    #[test]
    fn add_correct() {
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

    #[test]
    fn add_generic_correct() {
        assert!(add(&NumGeneric(1), &NumGeneric(2)) == NumGeneric(3));
        assert!(add(&NumGeneric(2), NumGeneric(4)) == NumGeneric(6));
        assert!(add(NumGeneric(3), &NumGeneric(6)) == NumGeneric(9));
        assert!(add(NumGeneric(4), NumGeneric(8)) == NumGeneric(12));

        assert!(add_assign(NumGeneric(3), &NumGeneric(6)) == NumGeneric(9));
        assert!(add_assign(NumGeneric(4), NumGeneric(8)) == NumGeneric(12));
    }
}
