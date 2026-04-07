//! Helper macro to check preconditions, printing compiler warnings on failure.
pub use crate::{none_or, ok_or, or, some_or};

/// Ensures that the condition is `true`, otherwise prints the
/// specified warning and returns `()` from the current function.
#[macro_export]
macro_rules! or {
    ($cond:expr, $($t:tt)*) => {
        if let false = $cond {
            return ::std::println!("cargo::warning={}", format_args!($($t)*));
        }
    };
}

/// Ensures that the value is [`Ok`], and if so evaluates to its inner value.
///
/// Otherwise, prints the specified warning and returns `()` from the current
/// function.
#[macro_export]
macro_rules! ok_or {
    ($value:expr, $why:pat => $($t:tt)*) => {
        match $value {
            ::std::result::Result::Ok(v) => v,
            ::std::result::Result::Err($why) => return ::std::println!("cargo::warning={}", format_args!($($t)*)),
        }
    };
}

/// Ensures that the value is [`Some`], and if so evaluates to its inner value.
///
/// Otherwise, prints the specified warning and returns `()` from the current
/// function.
#[macro_export]
macro_rules! some_or {
    ($value:expr, $($t:tt)*) => {
        match $value {
            ::std::option::Option::Some(v) => v,
            ::std::option::Option::None => return ::std::println!("cargo::warning={}", format_args!($($t)*)),
        }
    };
}

/// Ensures that the value is [`None`], otherwise, prints the specified warning
/// and returns `()` from the current function.
#[macro_export]
macro_rules! none_or {
    ($value:expr, $why:pat => $($t:tt)*) => {
        if let ::std::option::Option::Some($why) = $value {
            return ::std::println!("cargo::warning={}", format_args!($($t)*));
        }
    };
}
