/// Parses a slash argument from a context by its name and type.
#[macro_export]
macro_rules! parse_slash_argument {
    ($ctx:expr, $name:literal, $ty:ty) => {{
        let value = $ctx.option_value($name);
        <$ty as $crate::private::SlashArgOption<'_>>::try_extract(&$ctx, value)
    }};
}

/// Creates the base data needed for a slash argument.
///
/// This exists only to support the macro infrastructure and isn't considered
/// public API.
#[macro_export]
#[doc(hidden)]
macro_rules! create_slash_argument {
    (($($body:tt)*), $ty:ty, $($setter:tt)*) => {
        $crate::model::Parameter {
            $($body)*,
            required: <$ty as $crate::private::SlashArgOption<'_>>::REQUIRED,
            choices: <<$ty as $crate::private::SlashArgOption<'_>>::Required as $crate::SlashArg<'_>>::choices,
            #[allow(unnecessary_cast)]
            type_setter: |c| <<$ty as $crate::private::SlashArgOption<'_>>::Required as $crate::SlashArg<'_>>::set_options(c) $($setter)*,
        }
    };
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
