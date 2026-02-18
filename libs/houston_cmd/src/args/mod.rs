use std::borrow::Cow;

pub use houston_cmd_macros::ChoiceArg;
use serenity::builder::CreateCommandOption;
use serenity::model::application::{CommandOptionType, ResolvedValue};
use serenity::model::channel::{Attachment, GenericInteractionChannel, Message};
use serenity::model::guild::{PartialMember, Role};
use serenity::model::user::User;

use crate::context::Context;
use crate::error::Error;
use crate::model::Choice;

mod impls;
mod resolver;
mod str_arg;

pub use resolver::CommandOptionResolver;
pub use str_arg::FromStrArg;

/// A resolved option of a slash command.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ResolvedOption<'a> {
    /// The name of the option.
    ///
    /// This usually matches the name of the user-defined command function
    /// parameter.
    pub name: &'a str,
    /// The resolved value of the option.
    pub value: ResolvedValue<'a>,
}

/// Enables a type to be used as an argument in a
/// [`#[chat_command]`](crate::chat_command).
///
/// If the type already implements [`FromStr`](std::str::FromStr), you can use
/// [`crate::impl_slash_arg_via_from_str`] to implement this trait over
/// `from_str`.
pub trait SlashArg<'ctx>: Sized {
    /// Extracts the argument value.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the input is invalid for the value, parsing failed, or
    /// the argument value can otherwise not be used.
    fn extract(ctx: &Context<'ctx>, resolved: &ResolvedValue<'ctx>) -> Result<Self, Error<'ctx>>;

    /// Sets the options relevant to the parameter type.
    ///
    /// Notably, [`kind`](CreateCommandOption::kind) must be set, but limits may
    /// also be provided.
    fn set_options(option: CreateCommandOption<'_>) -> CreateCommandOption<'_>;

    /// Gets the choices, if this is intended to be a choice parameter.
    ///
    /// This serves only to build data to create the commands on Discord's end
    /// and isn't queried by the framework.
    fn choices() -> Cow<'static, [Choice]> {
        Cow::Borrowed(&[])
    }
}

/// Enables a choice-type argument in a
/// [`#[chat_command]`](crate::chat_command).
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
    fn extract(ctx: &Context<'ctx>, resolved: &ResolvedValue<'ctx>) -> Result<Self, Error<'ctx>> {
        match *resolved {
            ResolvedValue::Integer(index) => usize::try_from(index)
                .ok()
                .and_then(Self::from_index)
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

/// Enables a type to be loaded as a
/// [`#[context_command(user)]`](crate::context_command) parameter.
///
/// By default, this is implemented for [`&User`](User) and `(&User,
/// Option<&PartialMember>)`.
#[diagnostic::on_unimplemented(
    message = "this parameter type isn't supported for `user` context commands"
)]
pub trait UserContextArg<'ctx>: Sized {
    /// Extracts the argument value.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the input is invalid for the value, parsing failed, or
    /// the argument value can otherwise not be used.
    fn extract(
        ctx: &Context<'ctx>,
        user: &'ctx User,
        member: Option<&'ctx PartialMember>,
    ) -> Result<Self, Error<'ctx>>;
}

/// Enables a type to be loaded as a
/// [`#[context_command(message)]`](crate::context_command) parameter.
///
/// By default, this is implemented for [`&Message`](Message).
#[diagnostic::on_unimplemented(
    message = "this parameter type isn't supported for `message` context commands"
)]
pub trait MessageContextArg<'ctx>: Sized {
    /// Extracts the argument value.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the input is invalid for the value, parsing failed, or
    /// the argument value can otherwise not be used.
    fn extract(ctx: &Context<'ctx>, message: &'ctx Message) -> Result<Self, Error<'ctx>>;
}
