//! Helper macro to check preconditions, printing compiler warnings on failure.
use std::fmt;

pub use crate::{err, none_or, ok_or, or, some_or};

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

/// To be used by [`Result::map_err`].
///
/// Creates an [`Err`] and prints the message as a warning.
///
/// While the other functions in the [`ensure`](self) module are intended to be
/// transparent to the caller, this can be used when you explicitly want to
/// handle the failure case.
#[macro_export]
macro_rules! err {
    ($($t:tt)*) => {
        $crate::ensure::Err::new(::std::format_args!($($t)*))
    };
}

/// Error that has been printed as a warning.
#[derive(Clone)]
pub struct Err(String);

impl Err {
    /// Prints a warning and creates a new [`Err`] with that message.
    pub fn new(args: fmt::Arguments<'_>) -> Self {
        let value = args.to_string();
        println!("cargo::warning={args}");
        Self(value)
    }

    /// The error message.
    pub fn message(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Err {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.message(), f)
    }
}
