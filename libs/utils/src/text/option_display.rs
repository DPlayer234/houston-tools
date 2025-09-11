use std::fmt::{self, Display, Formatter};

/// Implements [`Display`] for an [`Option`], printing the value if [`Some`],
/// and printing nothing if [`None`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OptionDisplay<T>(Option<T>);

impl<T> OptionDisplay<T> {
    /// Creates a new [`Display`]-option.
    pub fn new(result: Option<T>) -> Self {
        Self(result)
    }
}

impl<T: Display> Display for OptionDisplay<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(item) => T::fmt(item, f),
            None => Ok(()),
        }
    }
}

/// Implements [`Display`] for a [`Result`], directly delegating to the
/// implementation of the active variant.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResultDisplay<T, E>(Result<T, E>);

impl<T, E> ResultDisplay<T, E> {
    /// Creates a new [`Display`]-result.
    pub fn new(result: Result<T, E>) -> Self {
        Self(result)
    }
}

impl<T: Display, E: Display> Display for ResultDisplay<T, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Ok(item) => T::fmt(item, f),
            Err(or) => E::fmt(or, f),
        }
    }
}

/// Extension trait for extended [`Display`].
pub trait DisplayExt {
    /// The type to display `Self` as.
    #[doc(hidden)]
    type DisplayTy: Display;

    /// Turns this value into something that can [`Display`] it.
    fn display(self) -> Self::DisplayTy;
}

impl<T: Display> DisplayExt for Option<T> {
    #[doc(hidden)]
    type DisplayTy = OptionDisplay<T>;

    /// Turns this [`Option`] into a [`Display`] type that displays the value if
    /// [`Some`] and displaying nothing if [`None`].
    fn display(self) -> Self::DisplayTy {
        OptionDisplay::new(self)
    }
}

impl<T: Display, E: Display> DisplayExt for Result<T, E> {
    #[doc(hidden)]
    type DisplayTy = ResultDisplay<T, E>;

    /// Turns this [`Result`] into a [`Display`] type that delegates to the
    /// implementation for the active variant.
    fn display(self) -> Self::DisplayTy {
        ResultDisplay::new(self)
    }
}
