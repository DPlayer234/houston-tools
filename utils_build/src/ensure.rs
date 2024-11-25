//! Helper macro to check preconditions, printing compiler warnings on failure.

/// Ensures that the condition is `true`, otherwise prints the
/// specified warning and returns `()` from the current function.
macro_rules! or {
    ($cond:expr, $($t:tt)*) => {
        if !$cond {
            return println!("cargo::warning={}", format_args!($($t)*));
        }
    };
}

/// Ensures that the value is [`Ok`], and if so evaluates to its inner value.
///
/// Otherwise, prints the specified warning and returns `()` from the current function.
macro_rules! ok_or {
    ($value:expr, $why:pat => $($t:tt)*) => {
        match $value {
            Ok(v) => v,
            Err($why) => return println!("cargo::warning={}", format_args!($($t)*)),
        }
    };
}

pub(crate) use or;
pub(crate) use ok_or;
