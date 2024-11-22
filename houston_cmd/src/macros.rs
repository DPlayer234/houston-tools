/// Parses a slash argument from a context by its name and type.
#[macro_export]
macro_rules! parse_slash_argument {
    ($ctx:expr, $name:literal, Option<$ty:ty>) => {
        if let Some(value) = $ctx.options().iter().find(|o| o.name == $name) {
            Some(<$ty as $crate::SlashArg>::extract(&$ctx, &value.value)?)
        } else {
            None
        }
    };
    ($ctx:expr, $name:literal, $ty:ty) => {{
        let Some(value) = $ctx.options().iter().find(|o| o.name == $name)
        else { return Err($crate::Error::structure_mismatch($ctx, "a required parameter is missing")) };

        <$ty as $crate::SlashArg>::extract(&$ctx, &value.value)?
    }};
}

/// Creates the base data needed for a slash argument.
///
/// This exists only to support the macro infrastructure and isn't considered public API.
#[macro_export]
#[doc(hidden)]
macro_rules! create_slash_argument {
    (Option<$ty:ty>, $($setter:tt)*) => {
        $crate::create_slash_argument!(@internal $ty, false, $($setter)*)
    };
    ($ty:ty, $($setter:tt)*) => {
        $crate::create_slash_argument!(@internal $ty, true, $($setter)*)
    };
    (@internal $ty:ty, $req:literal, $($setter:tt)*) => {
        $crate::model::Parameter {
            name: ::std::borrow::Cow::Borrowed(""),
            description: ::std::borrow::Cow::Borrowed(""),
            required: $req,
            autocomplete: ::core::option::Option::None,
            choices: <$ty as $crate::SlashArg>::choices,
            type_setter: |c| <$ty as $crate::SlashArg>::set_options(c) $($setter)*,
        }
    };
}

/// Implements the necessary traits to allow a [`UserContextArg`](crate::UserContextArg)
/// to actually work as a [`#[context_command]`](crate::context_command) parameter.
#[macro_export]
macro_rules! impl_user_context_arg {
    ($l:lifetime $ty:ty) => {
        impl<$l> $crate::ContextArg<$l> for $ty
        where
            $ty: $crate::UserContextArg<$l>,
        {
            const INVOKE: $crate::model::Invoke = $crate::model::Invoke::User(|_, _, _| unreachable!("do not call"));

            fn extract_user(
                ctx: &$crate::Context<$l>,
                user: &$l $crate::private::serenity::User,
                member: ::std::option::Option<&$l $crate::private::serenity::PartialMember>,
            ) -> ::std::result::Result<Self, $crate::Error<$l>> {
                $crate::UserContextArg::extract(ctx, user, member)
            }
        }
    };
}

/// Implements the necessary traits to allow a [`MessageContextArg`](crate::MessageContextArg)
/// to actually work as a [`#[context_command]`](crate::context_command) parameter.
#[macro_export]
macro_rules! impl_message_context_arg {
    ($l:lifetime $ty:ty) => {
        impl<$l> $crate::ContextArg<$l> for $ty
        where
            $ty: $crate::MessageContextArg<$l>,
        {
            const INVOKE: $crate::model::Invoke = $crate::model::Invoke::Message(|_, _| unreachable!("do not call"));

            fn extract_message(
                ctx: &$crate::Context<$l>,
                message: &$l $crate::private::serenity::Message,
            ) -> ::std::result::Result<Self, $crate::Error<$l>> {
                $crate::MessageContextArg::extract(ctx, message)
            }
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
