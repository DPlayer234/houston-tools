use std::fmt::{Debug, Display, Formatter, Result};

/// Struct created by [`from_fn`].
#[derive(Clone, Copy)]
pub struct FromFn<F>(F);

/// Creates an adhoc [`Display`] and [`Debug`] implementation from a function.
///
/// The function must behave as [`Display::fmt`] expects it to. That also means
/// returning an error when the formatter didn't is a logic error.
///
/// The [`Debug`] and [`Display`] output of the returned type will be exactly
/// the same.
///
/// # Examples
///
/// Creating an adhoc [`Display`] around an [`Option`]:
/// ```
/// let item = Some(0);
/// let fmt = utils::text::from_fn(|f| match item {
///     Some(value) => write!(f, "Some: {value}"),
///     None => write!(f, "None"),
/// });
/// # assert_eq!(fmt.to_string(), "Some: 0");
/// ```
pub fn from_fn<F>(f: F) -> FromFn<F>
where
    F: Fn(&mut Formatter<'_>) -> Result,
{
    FromFn(f)
}

impl<F> Display for FromFn<F>
where
    F: Fn(&mut Formatter<'_>) -> Result,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        (self.0)(f)
    }
}

impl<F> Debug for FromFn<F>
where
    F: Fn(&mut Formatter<'_>) -> Result,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        (self.0)(f)
    }
}

#[macro_export]
macro_rules! format_owned {
    ([$($n:ident = $cap:expr),* $(,)?], $($t:tt)*) => {{
        $(let $n = $cap;)*
        $crate::text::from_fn(move |f| ::std::fmt::Formatter::write_fmt(f, ::std::format_args!($($t)*)))
    }};
    ($($t:tt)*) => {
        $crate::text::from_fn(move |f| ::std::fmt::Formatter::write_fmt(f, ::std::format_args!($($t)*)))
    };
}
