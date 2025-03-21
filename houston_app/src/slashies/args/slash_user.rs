use houston_cmd::{Context, Error, SlashArg, UserContextArg};

use crate::helper::discord::Partial;
use crate::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct SlashUser<'a> {
    pub user: &'a User,
    pub member: Option<&'a PartialMember>,
}

impl<'ctx> SlashArg<'ctx> for SlashUser<'ctx> {
    fn extract(ctx: &Context<'ctx>, resolved: &ResolvedValue<'ctx>) -> Result<Self, Error<'ctx>> {
        match *resolved {
            ResolvedValue::User(user, member) => Ok(Self { user, member }),
            _ => Err(Error::structure_mismatch(*ctx, "expected User")),
        }
    }

    fn set_options(option: CreateCommandOption<'_>) -> CreateCommandOption<'_> {
        option.kind(CommandOptionType::User)
    }
}

impl<'ctx> UserContextArg<'ctx> for SlashUser<'ctx> {
    fn extract(
        _ctx: &Context<'ctx>,
        user: &'ctx User,
        member: Option<&'ctx PartialMember>,
    ) -> Result<Self, Error<'ctx>> {
        Ok(Self { user, member })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SlashMember<'a> {
    pub user: &'a User,
    pub member: Partial<&'a Member>,
}

impl<'ctx> SlashArg<'ctx> for SlashMember<'ctx> {
    fn extract(ctx: &Context<'ctx>, resolved: &ResolvedValue<'ctx>) -> Result<Self, Error<'ctx>> {
        match *resolved {
            ResolvedValue::User(user, Some(member)) => {
                return Ok(Self {
                    user,
                    member: Partial::Partial(member),
                });
            },
            // delegate to this method to get the correct error
            _ => drop(<&PartialMember as SlashArg>::extract(ctx, resolved)?),
        }

        // this is functionally unreachable
        Err(Error::structure_mismatch(*ctx, "expected Member"))
    }

    fn set_options(options: CreateCommandOption<'_>) -> CreateCommandOption<'_> {
        options.kind(CommandOptionType::User)
    }
}

impl<'ctx> UserContextArg<'ctx> for SlashMember<'ctx> {
    fn extract(
        ctx: &Context<'ctx>,
        user: &'ctx User,
        member: Option<&'ctx PartialMember>,
    ) -> Result<Self, Error<'ctx>> {
        let member = member.ok_or_else(|| Error::arg_invalid(*ctx, "unknown server member"))?;
        Ok(Self {
            user,
            member: Partial::Partial(member),
        })
    }
}

#[expect(dead_code, reason = "reserved for later use")]
impl SlashUser<'_> {
    pub fn display_name(&self) -> &str {
        self.member
            .and_then(|m| m.nick.as_deref())
            .unwrap_or_else(|| self.user.display_name())
    }

    pub fn face(&self) -> String {
        self.user.face()
    }
}

impl<'a> SlashMember<'a> {
    pub fn from_ctx(ctx: Context<'a>) -> Result<Self> {
        let member = ctx.member().context("member must be present")?;
        Ok(Self {
            user: ctx.user(),
            member: Partial::Full(member),
        })
    }

    pub fn nick(&self) -> Option<&str> {
        match self.member {
            Partial::Full(m) => m.nick.as_deref(),
            Partial::Partial(m) => m.nick.as_deref(),
        }
    }

    pub fn display_name(&self) -> &str {
        self.nick().unwrap_or_else(|| self.user.display_name())
    }

    pub fn face(&self) -> String {
        match self.member {
            Partial::Full(m) => m.face(),
            // PartialMember has no guild avatar
            Partial::Partial(_) => self.user.face(),
        }
    }
}

impl Mentionable for SlashUser<'_> {
    fn mention(&self) -> Mention {
        Mention::User(self.user.id)
    }
}

impl Mentionable for SlashMember<'_> {
    fn mention(&self) -> Mention {
        Mention::User(self.user.id)
    }
}
