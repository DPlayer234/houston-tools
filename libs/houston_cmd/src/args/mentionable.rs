use serenity::builder::CreateCommandOption;
use serenity::model::application::{CommandOptionType, ResolvedValue};
use serenity::model::channel::GenericInteractionChannel;
use serenity::model::guild::{PartialMember, Role};
use serenity::model::user::User;

use crate::{Context, Error, SlashArg};

/// A [mentionable][CommandOptionType::Mentionable] command parameter to be used
/// with [`chat_command`][crate::chat_command].
///
/// In essence, this allows a command to accept either a channel, role, or user.
#[derive(Debug, Clone, Copy)]
#[expect(missing_docs)]
// not #[non_exhaustive], seems fine to break this if it ever changes
pub enum Mentionable<'a> {
    Channel(&'a GenericInteractionChannel),
    Role(&'a Role),
    User(&'a User, Option<&'a PartialMember>),
}

impl<'ctx> SlashArg<'ctx> for Mentionable<'ctx> {
    fn extract(_ctx: &Context<'ctx>, resolved: &ResolvedValue<'ctx>) -> Result<Self, Error<'ctx>> {
        match *resolved {
            ResolvedValue::Channel(v) => Ok(Self::Channel(v)),
            ResolvedValue::Role(v) => Ok(Self::Role(v)),
            ResolvedValue::User(a, b) => Ok(Self::User(a, b)),
            _ => Err(Error::structure_mismatch("expected Channel, Role, or User")),
        }
    }

    fn set_options(option: CreateCommandOption<'_>) -> CreateCommandOption<'_> {
        option.kind(CommandOptionType::Mentionable)
    }
}
