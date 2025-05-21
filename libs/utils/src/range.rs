//! Provides numeric ranges with compile-time provided limits
//! with enforcement at both compile and runtime.
//!
//! The primary intent is to allow easy parsing of user range inputs.

use std::error::Error as StdError;
use std::ops::{Bound, RangeBounds};

/// An error that can occur when constructing bounded ranges.
#[derive(Debug, thiserror::Error)]
pub enum OutOfRange<T: RangeNum> {
    /// The provided value was below the `MIN`.
    #[error("value must be at least {min}, was {actual}")]
    BelowMin {
        /// The actual value provided.
        actual: T,
        /// The static minimum.
        min: T,
    },

    /// The provided value was above the `MAX`.
    #[error("value must be at most {max}, was {actual}")]
    AboveMax {
        /// The actual value provided.
        actual: T,
        /// The static maximum.
        max: T,
    },

    /// The low value was above the high value.
    /// This variant stores the provided low and high values.
    #[error("low ({low}) is greater than high ({high})")]
    LowAboveHigh { low: T, high: T },

    /// Parsing failed.
    #[error("expected range within limits [{min}..{max}]; {source}")]
    Parse {
        min: T,
        max: T,
        #[source]
        source: T::FromStrError,
    },
}

impl<T: RangeNum> OutOfRange<T> {
    const fn below_min(actual: T, min: T) -> Self {
        Self::BelowMin { actual, min }
    }
    const fn above_max(actual: T, max: T) -> Self {
        Self::AboveMax { actual, max }
    }
    const fn low_above_high(low: T, high: T) -> Self {
        Self::LowAboveHigh { low, high }
    }
    const fn parse(min: T, max: T, source: T::FromStrError) -> Self {
        Self::Parse { min, max, source }
    }
}

/// Const `?` as long as the error type matches.
macro_rules! try_const {
    ($e:expr) => {{
        match $e {
            Ok(v) => v,
            Err(e) => return Err(e),
        }
    }};
}

/// Marker trait for number types used within the range types of this module.
pub trait RangeNum {
    /// The error type for the [`std::str::FromStr`] implementation.
    type FromStrError: StdError + 'static;
}

macro_rules! impl_range {
    ($Type:ident, $Num:ty) => {
        #[doc = concat!("An inclusive range type using [`", stringify!($Num), "`] with static restrictions on the allowed values")]
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
            /// # Errors
            ///
            /// Returns [`Err`] if either value is outside the allowed range or
            /// the high value is less than the low value.
            ///
            /// # Example
            ///
            /// ```no_run
            /// # use utils::range::*;
            #[doc = concat!("let range = <", stringify!($Type), "<1, 10>>::new(4, 6);")]
            /// assert_eq!(range.unwrap().tuple(), (4, 6));
            /// ```
            pub const fn new(low: $Num, high: $Num) -> Result<Self, OutOfRange<$Num>> {
                const { Self::assert_valid(); }

                if low <= high {
                    let low = try_const!(Self::check(low));
                    let high = try_const!(Self::check(high));
                    Ok(Self(low, high))
                } else {
                    Err(OutOfRange::low_above_high(low, high))
                }
            }

            /// Checks if the value is within range.
            /// If within range, returns [`Ok`] with the input.
            ///
            /// # Errors
            ///
            /// Returns [`Err`] when the number is out of range.
            pub const fn check(n: $Num) -> Result<$Num, OutOfRange<$Num>> {
                const { Self::assert_valid(); }

                if n < MIN {
                    Err(OutOfRange::below_min(n, MIN))
                } else if n > MAX {
                    Err(OutOfRange::above_max(n, MAX))
                } else {
                    Ok(n)
                }
            }

            /// Gets the low end of this range.
            #[must_use]
            pub const fn low(self) -> $Num {
                self.0
            }

            /// Gets the high end of this range.
            #[must_use]
            pub const fn high(self) -> $Num {
                self.1
            }

            /// Gets a tuple of the components.
            #[must_use]
            pub const fn tuple(self) -> ($Num, $Num) {
                (self.0, self.1)
            }

            fn parse_part(s: &str) -> Result<$Num, OutOfRange<$Num>> {
                s.parse().map_err(|err| OutOfRange::parse(MIN, MAX, err))
            }

            #[track_caller]
            const fn assert_valid() {
                assert!(MIN <= MAX, "range type is invalid, MIN should be less than or equal to MAX");
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

        impl RangeNum for $Num {
            type FromStrError = <$Num as std::str::FromStr>::Err;
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
                use super::{OutOfRange, $Type};

                let valid = <$Type<1, 10>>::new(4, 6);
                let inverse = <$Type<1, 10>>::new(5, 4);
                let too_low = <$Type<1, 10>>::new(0, 8);
                let too_high = <$Type<1, 10>>::new(2, 11);

                assert!(matches!(valid.map($Type::tuple), Ok((4, 6))));
                assert!(matches!(
                    inverse,
                    Err(OutOfRange::LowAboveHigh { low: 5, high: 4 })
                ));
                assert!(matches!(
                    too_low,
                    Err(OutOfRange::BelowMin { actual: 0, min: 1 })
                ));
                assert!(matches!(
                    too_high,
                    Err(OutOfRange::AboveMax {
                        actual: 11,
                        max: 10
                    })
                ));
            }
        };
    }

    macro_rules! impl_parse_test {
        ($fn:ident, $Type:ident) => {
            #[test]
            fn $fn() {
                use std::str::FromStr as _;

                use super::$Type;

                let valid = <$Type<1, 10>>::from_str("4..6");
                let single = <$Type<1, 10>>::from_str("5");
                let low_only = <$Type<1, 10>>::from_str("4..");
                let high_only = <$Type<1, 10>>::from_str("..6");
                let all = <$Type<1, 10>>::from_str("..");

                assert!(matches!(valid.map($Type::tuple), Ok((4, 6))));
                assert!(matches!(single.map($Type::tuple), Ok((5, 5))));
                assert!(matches!(low_only.map($Type::tuple), Ok((4, 10))));
                assert!(matches!(high_only.map($Type::tuple), Ok((1, 6))));
                assert!(matches!(all.map($Type::tuple), Ok((1, 10))));
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

    impl_parse_test!(parse_range_u8, RangeU8);
    impl_parse_test!(parse_range_u16, RangeU16);
    impl_parse_test!(parse_range_u32, RangeU32);
    impl_parse_test!(parse_range_u64, RangeU64);
    impl_parse_test!(parse_range_u128, RangeU128);
    impl_parse_test!(parse_range_usize, RangeUsize);

    impl_parse_test!(parse_range_i8, RangeI8);
    impl_parse_test!(parse_range_i16, RangeI16);
    impl_parse_test!(parse_range_i32, RangeI32);
    impl_parse_test!(parse_range_i64, RangeI64);
    impl_parse_test!(parse_range_i128, RangeI128);
    impl_parse_test!(parse_range_isize, RangeIsize);
}
