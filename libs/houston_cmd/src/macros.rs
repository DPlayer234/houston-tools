/// Parses a slash argument from a context by its name and type, using its
/// [`SlashArg`] implementation.
///
/// If the type is [`Option<T>`], a missing parameter is accepted and the
/// macro returns `Ok(None)`. Otherwise, a missing parameter returns
/// `Err`.
///
/// # Errors
///
/// Returns `Err` if parsing of the present value fails or the parameter is
/// missing but the type isn't [`Option<T>`].
///
/// # Examples
///
/// ```
/// # use houston_cmd::{Context, Error, parse_slash_argument};
/// # fn example(ctx: Context<'_>) -> Result<(), Error<'_>> {
/// let value = parse_slash_argument!(ctx, "count", u32)?;
/// # Ok(())
/// # }
/// ```
///
/// Optional parameters can be transparently handled the same way:
///
/// ```
/// # use houston_cmd::{Context, Error, parse_slash_argument};
/// # fn example(ctx: Context<'_>) -> Result<(), Error<'_>> {
/// match parse_slash_argument!(ctx, "count", Option<u32>)? {
///     Some(found) => println!("has count: {found}"),
///     None => println!("has no count"),
/// }
/// # Ok(())
/// # }
/// ```
///
/// [`SlashArg`]: crate::SlashArg
#[macro_export]
macro_rules! parse_slash_argument {
    ($ctx:expr, $name:literal, $ty:ty) => {{
        let value = $ctx.option_value($name);
        <$ty as $crate::private::SlashArgOption<'_>>::try_extract(&$ctx, value)
    }};
}

/// Implements [`SlashArg`] via a type's [`FromStr`] implementations.
///
/// The implementation's [`FromStr::Err`] must be [`Error`] and `'static` to be
/// supported. This is the same requirement that [`FromStrArg`] has.
///
/// If you are dealing with a foreign type, you should use [`FromStrArg`].
///
/// [`SlashArg`]: crate::SlashArg
/// [`Error`]: std::error::Error
/// [`FromStr`]: std::str::FromStr
/// [`FromStr::Err`]: std::str::FromStr::Err
/// [`FromStrArg`]: crate::FromStrArg
#[macro_export]
macro_rules! impl_slash_arg_via_from_str {
    ($ty:ty) => {
        impl<'ctx> $crate::SlashArg<'ctx> for $ty {
            fn extract(
                ctx: &$crate::Context<'ctx>,
                resolved: &$crate::private::serenity::ResolvedValue<'ctx>,
            ) -> ::std::result::Result<Self, $crate::Error<'ctx>> {
                $crate::FromStrArg::extract(ctx, resolved).map(|v| v.0)
            }

            fn set_options(
                option: $crate::private::serenity::CreateCommandOption<'_>,
            ) -> $crate::private::serenity::CreateCommandOption<'_> {
                $crate::FromStrArg::<Self>::set_options(option)
            }
        }
    };
}
