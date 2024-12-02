/// Parses a slash argument from a context by its name and type.
#[macro_export]
macro_rules! parse_slash_argument {
    ($ctx:expr, $name:literal, Option<$ty:ty>) => {
        match $ctx.options().iter().find(|o| o.name == $name) {
            Some(value) => Some(<$ty as $crate::SlashArg>::extract(&$ctx, &value.value)?),
            None => None,
        }
    };
    ($ctx:expr, $name:literal, $ty:ty) => {{
        match $ctx.options().iter().find(|o| o.name == $name) {
            Some(value) => <$ty as $crate::SlashArg>::extract(&$ctx, &value.value)?,
            None => return Err($crate::Error::structure_mismatch($ctx, "a required parameter is missing")),
        }
    }};
}

/// Creates the base data needed for a slash argument.
///
/// This exists only to support the macro infrastructure and isn't considered public API.
#[macro_export]
#[doc(hidden)]
macro_rules! create_slash_argument {
    (($($body:tt)*), Option<$ty:ty>, $($setter:tt)*) => {
        $crate::create_slash_argument!(@internal ($($body)*), $ty, false, $($setter)*)
    };
    (($($body:tt)*), $ty:ty, $($setter:tt)*) => {
        $crate::create_slash_argument!(@internal ($($body)*), $ty, true, $($setter)*)
    };
    (@internal ($($body:tt)*), $ty:ty, $req:literal, $($setter:tt)*) => {
        $crate::model::Parameter {
            $($body)*,
            required: $req,
            choices: <$ty as $crate::SlashArg>::choices,
            type_setter: |c| <$ty as $crate::SlashArg>::set_options(c) $($setter)*,
        }
    };
}

/// Implements [`SlashArg`](crate::SlashArg) via a type's [`FromStr`](std::str::FromStr) implementations.
#[macro_export]
macro_rules! impl_slash_arg_via_from_str {
    ($ty:ty) => {
        impl<'ctx> $crate::SlashArg<'ctx> for $ty {
            fn extract(
                ctx: &$crate::Context<'ctx>,
                resolved: &$crate::private::serenity::ResolvedValue<'ctx>,
            ) -> ::std::result::Result<Self, $crate::Error<'ctx>> {
                match resolved {
                    $crate::private::serenity::ResolvedValue::String(value) => ::std::str::FromStr::from_str(value)
                        .map_err(|e| $crate::Error::argument_parse(*ctx, Some((*value).to_owned()), e)),
                    _ => Err($crate::Error::structure_mismatch(*ctx, "expected string argument")),
                }
            }

            fn set_options(
                option: $crate::private::serenity::CreateCommandOption<'_>,
            ) -> $crate::private::serenity::CreateCommandOption<'_> {
                option.kind($crate::private::serenity::CommandOptionType::String)
            }
        }
    };
}
