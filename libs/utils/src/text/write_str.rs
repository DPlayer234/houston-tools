//! Module to allow writing to [`String`]s without having to handle the
//! unreachable error case.

use std::fmt::{Arguments, Write};

/// Infallible [`Write`] equivalent.
///
/// Intended for using the [`write`] macro with [`String`] without having to
/// handle the invalid error case.
///
/// This trait does not provide functions such as `write_str`. Instead, use
/// [`String::push_str`] or similar.
pub trait WriteStr {
    /// Glue for usage of the [`write!`] macro with implementors of this trait.
    ///
    /// This method should generally not be invoked manually, but rather through
    /// the [`write!`] macro itself.
    ///
    /// This function may panic when a formatting trait implementation returns
    /// an error.
    fn write_fmt(&mut self, args: Arguments<'_>);
}

impl WriteStr for String {
    fn write_fmt(&mut self, args: Arguments<'_>) {
        #[cold]
        #[track_caller]
        fn fail_write_fmt() {
            panic!(
                "a formatting trait implementation returned an error when writing to a string cannot fail"
            );
        }

        if Write::write_fmt(self, args).is_err() {
            fail_write_fmt();
        }
    }
}

impl<W: WriteStr> WriteStr for &mut W {
    fn write_fmt(&mut self, args: Arguments<'_>) {
        (**self).write_fmt(args);
    }
}
