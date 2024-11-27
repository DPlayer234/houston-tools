use houston_cmd::{Context, Error, SlashArg, UserContextArg};

use crate::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct SlashUser<'a> {
    pub user: &'a User,
    pub member: Option<&'a PartialMember>,
}

impl<'ctx> SlashArg<'ctx> for SlashUser<'ctx> {
    fn extract(
        ctx: &Context<'ctx>,
        resolved: &ResolvedValue<'ctx>,
    ) -> Result<Self, Error<'ctx>> {
        match *resolved {
            ResolvedValue::User(user, member) => Ok(Self { user, member }),
            _ => Err(Error::structure_mismatch(*ctx, "expected User"))
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
    pub member: &'a PartialMember,
}

#[serenity::async_trait]
impl<'ctx> SlashArg<'ctx> for SlashMember<'ctx> {
    fn extract(
        ctx: &Context<'ctx>,
        resolved: &ResolvedValue<'ctx>,
    ) -> Result<Self, Error<'ctx>> {
        match *resolved {
            ResolvedValue::User(user, Some(member)) => return Ok(Self { user, member }),
            // delegate to this method to get the correct error
            _ => drop(<&PartialMember as SlashArg>::extract(ctx, resolved)?)
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
        Ok(Self { user, member })
    }
}

macro_rules! impl_shared_user_fn {
    ($l:lifetime => $($t:tt)*) => {
        #[allow(dead_code, reason = "shared methods")]
        impl<$l> SlashUser<$l> {
            $($t)*
        }
        #[allow(dead_code, reason = "shared methods")]
        impl<$l> SlashMember<$l> {
            $($t)*
        }
    };
}

impl_shared_user_fn! { 'a =>
    pub fn display_name(&self) -> &str {
        self.member()
            .and_then(|m| m.nick.as_deref())
            .unwrap_or_else(|| self.user.display_name())
    }

    pub fn face(&self) -> String {
        // CMBK: PartialMember has no guild avatar field
        self.user.face()
    }
}

impl<'a> SlashUser<'a> {
    fn member(&self) -> Option<&'a PartialMember> {
        self.member
    }
}

impl<'a> SlashMember<'a> {
    fn member(&self) -> Option<&'a PartialMember> {
        Some(self.member)
    }
}

impl<'a> From<SlashMember<'a>> for SlashUser<'a> {
    fn from(value: SlashMember<'a>) -> Self {
        Self {
            user: value.user,
            member: Some(value.member),
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
