use std::fmt::{Display, Debug};
use std::ops::{RangeBounds, Bound};

#[derive(Debug, Clone)]
pub enum OutOfRange<T> {
    BelowMin(T),
    AboveMax(T),
    LowAboveHigh(T, T),
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

macro_rules! impl_range {
    ($Type:ident, $Num:ty) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[must_use]
        pub struct $Type<const MIN: $Num, const MAX: $Num>($Num, $Num);

        impl<const MIN: $Num, const MAX: $Num> $Type<MIN, MAX> {
            pub const ALL: Self = Self(MIN, MAX);
            pub const MIN: $Num = MIN;
            pub const MAX: $Num = MAX;

            pub fn new(low: $Num, high: $Num) -> Result<Self, OutOfRange<$Num>> {
                let low = Self::check(low)?;
                let high = Self::check(high)?;
                if low <= high {
                    Ok(Self(low, high))
                } else {
                    Err(OutOfRange::LowAboveHigh(low, high))
                }
            }

            pub fn check(n: $Num) -> Result<$Num, OutOfRange<$Num>> {
                if n < MIN {
                    Err(OutOfRange::BelowMin(MIN))
                } else if n > MAX {
                    Err(OutOfRange::AboveMax(MAX))
                } else {
                    Ok(n)
                }
            }

            pub fn low(self) -> $Num {
                self.0
            }

            pub fn high(self) -> $Num {
                self.1
            }

            pub fn tuple(self) -> ($Num, $Num) {
                (self.0, self.1)
            }

            fn parse_part(s: &str) -> Result<$Num, OutOfRange<$Num>> {
                s.parse().map_err(|_| OutOfRange::Parse(MIN, MAX))
            }
        }

        impl<const MIN: $Num, const MAX: $Num> TryFrom<($Num, $Num)> for $Type<MIN, MAX> {
            type Error = OutOfRange<$Num>;

            fn try_from(value: ($Num, $Num)) -> Result<Self, Self::Error> {
                Self::new(value.0, value.1)
            }
        }

        impl<const MIN: $Num, const MAX: $Num> std::str::FromStr for $Type<MIN, MAX> {
            type Err = OutOfRange<$Num>;

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
