use houston_cmd::{Context, Error, SlashArg, UserContextArg};

use crate::helper::discord::{Partial, guild_avatar_url};
use crate::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct SlashUser<'a> {
    pub user: &'a User,
    pub member: Option<&'a PartialMember>,
    guild_id: Option<GuildId>,
}

impl<'ctx> SlashArg<'ctx> for SlashUser<'ctx> {
    fn extract(ctx: &Context<'ctx>, resolved: &ResolvedValue<'ctx>) -> Result<Self, Error<'ctx>> {
        match *resolved {
            ResolvedValue::User(user, member) => Ok(Self {
                user,
                member,
                guild_id: ctx.guild_id(),
            }),
            _ => Err(Error::structure_mismatch(*ctx, "expected User")),
        }
    }

    fn set_options(option: CreateCommandOption<'_>) -> CreateCommandOption<'_> {
        option.kind(CommandOptionType::User)
    }
}

impl<'ctx> UserContextArg<'ctx> for SlashUser<'ctx> {
    fn extract(
        ctx: &Context<'ctx>,
        user: &'ctx User,
        member: Option<&'ctx PartialMember>,
    ) -> Result<Self, Error<'ctx>> {
        Ok(Self {
            user,
            member,
            guild_id: ctx.guild_id(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SlashMember<'a> {
    pub user: &'a User,
    pub member: Partial<&'a Member>,
    guild_id: GuildId,
}

impl<'ctx> SlashArg<'ctx> for SlashMember<'ctx> {
    fn extract(ctx: &Context<'ctx>, resolved: &ResolvedValue<'ctx>) -> Result<Self, Error<'ctx>> {
        match *resolved {
            ResolvedValue::User(user, Some(member)) => {
                return Ok(Self {
                    user,
                    member: Partial::Partial(member),
                    guild_id: ctx.guild_id().unwrap_or_default(),
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
            guild_id: ctx.guild_id().unwrap_or_default(),
        })
    }
}

impl SlashUser<'_> {
    #[expect(dead_code, reason = "reserved for later use")]
    pub fn display_name(&self) -> &str {
        self.member
            .and_then(|m| m.nick.as_deref())
            .unwrap_or_else(|| self.user.display_name())
    }

    pub fn face(&self) -> String {
        if let Some(hash) = self.member.and_then(|m| m.avatar.as_ref()) {
            guild_avatar_url(self.user.id, self.guild_id.unwrap_or_default(), hash)
        } else {
            self.user.face()
        }
    }
}

impl<'a> SlashMember<'a> {
    pub fn from_ctx(ctx: Context<'a>) -> Result<Self> {
        let member = ctx.member().context("member must be present")?;
        Ok(Self {
            user: ctx.user(),
            member: Partial::Full(member),
            guild_id: member.guild_id,
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
        let hash = match self.member {
            Partial::Full(m) => &m.avatar,
            Partial::Partial(m) => &m.avatar,
        };

        if let Some(hash) = hash {
            guild_avatar_url(self.user.id, self.guild_id, hash)
        } else {
            self.user.face()
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
