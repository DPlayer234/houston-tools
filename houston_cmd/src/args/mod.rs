use std::borrow::Cow;

use serenity::builder::CreateCommandOption;
use serenity::model::application::{CommandOptionType, ResolvedValue};
use serenity::model::channel::{Attachment, Message, PartialChannel};
use serenity::model::guild::{PartialMember, Role};
use serenity::model::user::User;

mod impls;

use crate::context::Context;
use crate::error::Error;
use crate::model::Choice;

pub use ::houston_cmd_macros::ChoiceArg;

/// Enables a type to be used as an argument in a [`#[chat_command]`](crate::chat_command).
///
/// If the type already implements [`FromStr`](std::str::FromStr), you can use
/// [`crate::impl_slash_arg_via_from_str`] to implement this trait over `from_str`.
pub trait SlashArg<'ctx>: Sized {
    /// Extracts the argument value.
    fn extract(
        ctx: &Context<'ctx>,
        resolved: &ResolvedValue<'ctx>,
    ) -> Result<Self, Error<'ctx>>;

    /// Sets the options relevant to the parameter type.
    ///
    /// Notably, [`kind`](CreateCommandOption::kind) must be set, but limits may also be provided.
    fn set_options(option: CreateCommandOption<'_>) -> CreateCommandOption<'_>;

    /// Gets the choices, if this is intended to be a choice parameter.
    ///
    /// This serves only to build data to create the commands on Discord's end and isn't queried by the framework.
    fn choices() -> Cow<'static, [Choice]> { Cow::Borrowed(&[]) }
}

/// Enables a choice-type argument in a [`#[chat_command]`](crate::chat_command).
///
/// This will auto-implement [`SlashArg`].
pub trait ChoiceArg: Sized {
    /// Gets the list of choices.
    ///
    /// [`SlashArg::choices`] will return this value.
    fn list() -> Cow<'static, [Choice]>;

    /// Gets a value by its choice index.
    fn from_index(index: usize) -> Option<Self>;
}

impl<'ctx, T> SlashArg<'ctx> for T
where
    T: ChoiceArg,
{
    fn extract(
        ctx: &Context<'ctx>,
        resolved: &ResolvedValue<'ctx>,
    ) -> Result<Self, Error<'ctx>> {
        match resolved {
            ResolvedValue::Integer(index) => Self::from_index(*index as usize)
                .ok_or_else(|| Error::structure_mismatch(*ctx, "invalid choice index")),
            _ => Err(Error::structure_mismatch(*ctx, "expected integer")),
        }
    }

    fn choices() -> Cow<'static, [Choice]> {
        <Self as ChoiceArg>::list()
    }

    fn set_options(option: CreateCommandOption<'_>) -> CreateCommandOption<'_> {
        option.kind(CommandOptionType::Integer)
    }
}

/// Enables a type to be loaded as a [`#[context_command(user)]`](crate::context_command) parameter.
///
/// By default, this is implemented for [`&User`](User).
pub trait UserContextArg<'ctx>: Sized {
    fn extract(
        ctx: &Context<'ctx>,
        user: &'ctx User,
        member: Option<&'ctx PartialMember>,
    ) -> Result<Self, Error<'ctx>>;
}

/// Enables a type to be loaded as a [`#[context_command(message)]`](crate::context_command) parameter.
///
/// By default, this is implemented for [`&Message`](Message).
pub trait MessageContextArg<'ctx>: Sized {
    fn extract(
        ctx: &Context<'ctx>,
        message: &'ctx Message,
    ) -> Result<Self, Error<'ctx>>;
}
