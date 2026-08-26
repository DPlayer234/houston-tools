use serenity::builder::CreateCommandOption;
use serenity::model::application::{CommandOptionType, ResolvedValue};
use serenity::model::guild::{PartialMember, Role};
use serenity::model::mention::Mention;
use serenity::model::user::User;
use serenity::prelude::Mentionable as MentionableTrait;

use crate::{Context, Error, SlashArg};

/// A [mentionable][CommandOptionType::Mentionable] command parameter to be used
/// with [`chat_command`][crate::chat_command].
///
/// In essence, this allows a command to accept either a user or role.
#[derive(Debug, Clone, Copy)]
#[expect(missing_docs)]
// not #[non_exhaustive], seems fine to break this if it ever changes
pub enum Mentionable<'a> {
    User(&'a User, Option<&'a PartialMember>),
    Role(&'a Role),
}

impl<'ctx> SlashArg<'ctx> for Mentionable<'ctx> {
    fn extract(_ctx: &Context<'ctx>, resolved: &ResolvedValue<'ctx>) -> Result<Self, Error<'ctx>> {
        match *resolved {
            ResolvedValue::User(a, b) => Ok(Self::User(a, b)),
            ResolvedValue::Role(v) => Ok(Self::Role(v)),
            _ => Err(Error::structure_mismatch("expected User or Role")),
        }
    }

    fn set_options(option: CreateCommandOption<'_>) -> CreateCommandOption<'_> {
        option.kind(CommandOptionType::Mentionable)
    }
}

impl MentionableTrait for Mentionable<'_> {
    fn mention(&self) -> Mention {
        match *self {
            Mentionable::User(u, _) => u.mention(),
            Mentionable::Role(r) => r.mention(),
        }
    }
}
