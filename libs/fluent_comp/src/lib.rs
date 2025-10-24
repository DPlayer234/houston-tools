use std::fmt;

#[doc(hidden)]
pub mod private;

pub use fluent_comp_macros::bundle;

#[macro_export]
macro_rules! lang {
    ( $L:ident: $default:ident $(,$rest:ident)* ) => {
        #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::default::Default)]
        #[allow(non_camel_case_types)]
        pub enum $L {
            #[default] $default,
            $($rest,)*
        }

        impl ::core::str::FromStr for $L {
            type Err = ();

            fn from_str(s: &::core::primitive::str) -> ::core::result::Result<Self, ()> {
                if s.starts_with(stringify!($default)) {
                    ::core::result::Result::Ok(Self::$default)
                } else $( if s.starts_with(stringify!($rest)) {
                    ::core::result::Result::Ok(Self::$rest)
                } else )* {
                    ::core::result::Result::Err(())
                }
            }
        }
    };
}

pub trait FluentInt: fmt::Display {
    fn to_switch_value(&self) -> i8;
}

pub trait FluentStr: fmt::Display {
    fn to_switch_value(&self) -> &str;
}

#[expect(clippy::cast_possible_truncation)]
#[expect(clippy::cast_possible_wrap)]
mod impl_fluent_value {
    use core::fmt::Display;

    use super::{FluentInt, FluentStr};

    macro_rules! impl_fluent_num_signed {
        ($($T:ty)*) => {
            $(
                impl FluentInt for $T {
                    fn to_switch_value(&self) -> i8 {
                        (*self).clamp(-128, 127) as i8
                    }
                }
            )*
        };
    }

    macro_rules! impl_fluent_num_unsigned {
        ($($T:ty)*) => {
            $(
                impl FluentInt for $T {
                    fn to_switch_value(&self) -> i8 {
                        (*self).clamp(0, 127) as i8
                    }
                }
            )*
        };
    }

    impl_fluent_num_signed!(i8 i16 i32 i64 i128 isize);
    impl_fluent_num_unsigned!(u8 u16 u32 u64 u128 usize);

    impl<T: FluentInt> FluentInt for &T {
        fn to_switch_value(&self) -> i8 {
            (**self).to_switch_value()
        }
    }

    impl<T: AsRef<str> + Display> FluentStr for T {
        fn to_switch_value(&self) -> &str {
            self.as_ref()
        }
    }
}
