/// Creates a [`Display`](std::fmt::Display) value with [`format_args`] syntax
/// that tries to own its captures.
///
/// Optionally, you may specify how additional named captures upfront:
///
/// ```
/// let data = vec![0, 1, 2];
/// // capture a clone of `data` as `c`
/// let fmt = utils::format_owned!([c = data.clone()], "data is {c:?}");
/// println!("{fmt}");
/// ```
///
/// This macro returns a [`FromFn`](std::fmt::FromFn).
#[macro_export]
macro_rules! format_owned {
    ([$($n:ident = $cap:expr),* $(,)?], $($t:tt)*) => {{
        $(let $n = $cap;)*
        ::std::fmt::from_fn(move |f| ::std::fmt::Formatter::write_fmt(f, ::std::format_args!($($t)*)))
    }};
    ($($t:tt)*) => {
        ::std::fmt::from_fn(move |f| ::std::fmt::Formatter::write_fmt(f, ::std::format_args!($($t)*)))
    };
}
