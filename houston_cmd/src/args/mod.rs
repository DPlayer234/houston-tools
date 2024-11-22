use std::borrow::Cow;

use serenity::builder::CreateCommandOption;
use serenity::model::application::{CommandOptionType, ResolvedValue};
use serenity::model::channel::{Attachment, Message, PartialChannel};
use serenity::model::guild::{PartialMember, Role};
use serenity::model::user::User;

mod impls;

use crate::context::Context;
use crate::error::Error;
use crate::model::{Choice, Invoke};

pub use ::houston_cmd_macros::ChoiceArg;

pub trait SlashArg<'ctx>: Sized {
    fn extract(
        ctx: &Context<'ctx>,
        resolved: &ResolvedValue<'ctx>,
    ) -> Result<Self, Error<'ctx>>;

    fn choices() -> Cow<'static, [Choice]> { Cow::Borrowed(&[]) }
    fn set_options(option: CreateCommandOption<'_>) -> CreateCommandOption<'_>;
}

pub trait ChoiceArg: Sized {
    fn list() -> Cow<'static, [Choice]>;
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

#[doc(hidden)]
pub trait ContextArg<'ctx>: Sized {
    const INVOKE: Invoke;

    fn extract_user(
        _ctx: &Context<'ctx>,
        _user: &'ctx User,
        _member: Option<&'ctx PartialMember>,
    ) -> Result<Self, Error<'ctx>> {
        unreachable!()
    }

    fn extract_message(
        _ctx: &Context<'ctx>,
        _message: &'ctx Message,
    ) -> Result<Self, Error<'ctx>> {
        unreachable!()
    }
}

pub trait UserContextArg<'ctx>: Sized {
    fn extract(
        ctx: &Context<'ctx>,
        user: &'ctx User,
        member: Option<&'ctx PartialMember>,
    ) -> Result<Self, Error<'ctx>>;
}

pub trait MessageContextArg<'ctx>: Sized {
    fn extract(
        ctx: &Context<'ctx>,
        message: &'ctx Message,
    ) -> Result<Self, Error<'ctx>>;
}
