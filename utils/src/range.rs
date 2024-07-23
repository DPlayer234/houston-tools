use std::fmt::{Display, Debug};
use std::ops::{RangeBounds, Bound};

/// An error that can occur when constructing bounded ranges.
#[derive(Debug, Clone)]
pub enum OutOfRange<T> {
    /// The provided value was below the `MIN`.
    /// This variant stores the `MIN`.
    BelowMin(T),
    /// The provided value was above the `MAX`.
    /// This variant stores the `MAX`.
    AboveMax(T),
    /// The low value was above the high value.
    /// This variant stores the provided low and high values.
    LowAboveHigh(T, T),
    /// Parsing failed.
    /// This variant stores the static limits.
    Parse(T, T),
}

impl<T: Display + Debug> std::error::Error for OutOfRange<T> {}

impl<T: Display> Display for OutOfRange<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutOfRange::BelowMin(min) => write!(f, "Value must be at least {min}."),
            OutOfRange::AboveMax(max) => write!(f, "Value must be at most {max}."),
            OutOfRange::LowAboveHigh(l, h) => write!(f, "Low ({l}) is greater than high ({h})."),
            OutOfRange::Parse(min, max) => write!(f, "Expected a number or range within `[{min}..{max}]`."),
        }
    }
}

macro_rules! try_const {
    ($e:expr) => {{
        match $e {
            Ok(v) => v,
            Err(e) => return Err(e),
        }
    }};
}

macro_rules! impl_range {
    ($Type:ident, $Num:ty) => {
        /// An inclusive range type with static restrictions on the allowed values.
        ///
        /// This type is particularly useful when dealing with user input.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[must_use]
        pub struct $Type<const MIN: $Num, const MAX: $Num>($Num, $Num);

        impl<const MIN: $Num, const MAX: $Num> $Type<MIN, MAX> {
            /// Gets a range spanning `MIN` to `MAX`.
            pub const ALL: Self = {
                Self::assert_valid();
                Self(MIN, MAX)
            };

            /// Gets the minimum value.
            pub const MIN: $Num = MIN;

            /// Gets the maximum value.
            pub const MAX: $Num = MAX;

            /// Creates a new bounded range, both values being inclusive.
            ///
            /// Returns an error if either value is outside the allowed range or
            /// the high value is less than the low value.
            ///
            /// # Example
            ///
            /// ```no_run
            /// # use utils::range::RangeU8;
            /// let range = <RangeU8<1, 10>>::new(4, 6);
            /// assert_eq!(range.unwrap().tuple(), (4, 6));
            /// ```
            pub const fn new(low: $Num, high: $Num) -> Result<Self, OutOfRange<$Num>> {
                const { Self::assert_valid(); }

                let low = try_const!(Self::check(low));
                let high = try_const!(Self::check(high));
                if low <= high {
                    Ok(Self(low, high))
                } else {
                    Err(OutOfRange::LowAboveHigh(low, high))
                }
            }

            /// Checks if the value is within range.
            ///
            /// If within range, returns the same number.
            /// Otherwise, returns an error.
            pub const fn check(n: $Num) -> Result<$Num, OutOfRange<$Num>> {
                const { Self::assert_valid(); }

                if n < MIN {
                    Err(OutOfRange::BelowMin(MIN))
                } else if n > MAX {
                    Err(OutOfRange::AboveMax(MAX))
                } else {
                    Ok(n)
                }
            }

            /// Gets the low end of this range.
            pub const fn low(self) -> $Num {
                self.0
            }

            /// Gets the high end of this range.
            pub const fn high(self) -> $Num {
                self.1
            }

            /// Gets a tuple of the components.
            pub const fn tuple(self) -> ($Num, $Num) {
                (self.0, self.1)
            }

            fn parse_part(s: &str) -> Result<$Num, OutOfRange<$Num>> {
                s.parse().map_err(|_| OutOfRange::Parse(MIN, MAX))
            }

            const fn assert_valid() {
                assert!(MIN <= MAX);
            }
        }

        impl<const MIN: $Num, const MAX: $Num> TryFrom<($Num, $Num)> for $Type<MIN, MAX> {
            type Error = OutOfRange<$Num>;

            fn try_from(value: ($Num, $Num)) -> Result<Self, Self::Error> {
                Self::new(value.0, value.1)
            }
        }

        impl<const MIN: $Num, const MAX: $Num> From<$Type<MIN, MAX>> for ($Num, $Num) {
            fn from(value: $Type<MIN, MAX>) -> ($Num, $Num) {
                value.tuple()
            }
        }

        impl<const MIN: $Num, const MAX: $Num> std::str::FromStr for $Type<MIN, MAX> {
            type Err = OutOfRange<$Num>;

            /// Parses a range from a string.
            ///
            /// The expected format is either:
            /// - just a number, which sets both low and high to that number,
            /// - `low..high`, setting both parts,
            /// - `low..`, setting the low part and using `MAX` as high,
            /// - `..high`, setting the high part and using `MIN` as low, or
            /// - `..`, returning [`Self::ALL`].
            ///
            /// This can fail for the same reasons as [`Self::new`].
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.split_once("..") {
                    Some((min, max)) => {
                        Self::new(
                            if min.is_empty() { MIN } else { Self::parse_part(min)? },
                            if max.is_empty() { MAX } else { Self::parse_part(max)? },
                        )
                    }
                    None => {
                        let n: $Num = Self::parse_part(s)?;
                        Self::new(n, n)
                    }
                }
            }
        }

        impl<const MIN: $Num, const MAX: $Num> RangeBounds<$Num> for $Type<MIN, MAX> {
            fn start_bound(&self) -> Bound<&$Num> {
                Bound::Included(&self.0)
            }

            fn end_bound(&self) -> Bound<&$Num> {
                Bound::Included(&self.1)
            }
        }
    };
}

impl_range!(RangeU8, u8);
impl_range!(RangeU16, u16);
impl_range!(RangeU32, u32);
impl_range!(RangeU64, u64);
impl_range!(RangeU128, u128);
impl_range!(RangeUsize, usize);

impl_range!(RangeI8, i8);
impl_range!(RangeI16, i16);
impl_range!(RangeI32, i32);
impl_range!(RangeI64, i64);
impl_range!(RangeI128, i128);
impl_range!(RangeIsize, isize);

#[cfg(test)]
mod test {
    macro_rules! impl_test {
        ($fn:ident, $Type:ident) => {
            #[test]
            fn $fn() {
                use super::{$Type, OutOfRange};

                let valid = <$Type<1, 10>>::new(4, 6);
                let inverse = <$Type<1, 10>>::new(5, 4);
                let too_low = <$Type<1, 10>>::new(0, 8);
                let too_high = <$Type<1, 10>>::new(2, 11);

                assert!(matches!(valid.map($Type::tuple), Ok((4, 6))));
                assert!(matches!(inverse, Err(OutOfRange::LowAboveHigh(5, 4))));
                assert!(matches!(too_low, Err(OutOfRange::BelowMin(1))));
                assert!(matches!(too_high, Err(OutOfRange::AboveMax(10))));
            }
        };
    }

    impl_test!(range_u8, RangeU8);
    impl_test!(range_u16, RangeU16);
    impl_test!(range_u32, RangeU32);
    impl_test!(range_u64, RangeU64);
    impl_test!(range_u128, RangeU128);
    impl_test!(range_usize, RangeUsize);

    impl_test!(range_i8, RangeI8);
    impl_test!(range_i16, RangeI16);
    impl_test!(range_i32, RangeI32);
    impl_test!(range_i64, RangeI64);
    impl_test!(range_i128, RangeI128);
    impl_test!(range_isize, RangeIsize);
}
