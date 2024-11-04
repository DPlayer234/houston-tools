//! Module to allow writing to [`String`]s without having to handle the unreachable error case.

use std::fmt::{Arguments, Write};

// re-export these macros so they are usable with a wildcard import
pub use crate::{write_str, writeln_str};

/// [`Write`] trait equivalent for the [`write_str`] macro.
pub trait WriteStr {
    /// Writes the arguments to the buffer.
    fn write_str_fmt(&mut self, args: Arguments<'_>);

    /// Writes the arguments to the buffer, followed by a line-feed character (`\n`).
    fn writeln_str_fmt(&mut self, args: Arguments<'_>);
}

impl WriteStr for String {
    fn write_str_fmt(&mut self, args: Arguments<'_>) {
        #[cold]
        #[track_caller]
        fn fail() {
            panic!("write_fmt failed unexpectedly even though the buffer never returns an error");
        }

        let result = Write::write_fmt(self, args);
        if cfg!(debug_assertions) && result.is_err() {
            fail();
        }
    }

    fn writeln_str_fmt(&mut self, args: Arguments<'_>) {
        self.write_str_fmt(args);
        self.push('\n');
    }
}

/// Similar to [`write`], except it calls a method named `write_str_fmt`
/// and is generally intended to be infallible.
///
/// The buffer would generally be [`String`] and [`WriteStr`] should be imported.
#[macro_export]
macro_rules! write_str {
    ($buf:expr, $($t:tt)*) => {
        $buf.write_str_fmt(::std::format_args!($($t)*))
    };
}

/// Similar to [`writeln`], except it calls a method named `writeln_str_fmt`
/// and is generally intended to be infallible.
///
/// The buffer would generally be [`String`] and [`WriteStr`] should be imported.
#[macro_export]
macro_rules! writeln_str {
    ($buf:expr, $($t:tt)*) => {
        $buf.writeln_str_fmt(::std::format_args!($($t)*))
    };
}
