#![expect(dead_code)]

use anyhow::Context;
use poise::{SlashArgError, SlashArgument};

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct SlashUser {
    pub user: User,
    pub member: Option<PartialMember>,
}

#[serenity::async_trait]
impl SlashArgument for SlashUser {
    async fn extract(
        _ctx: &serenity::gateway::client::Context,
        _interaction: &CommandInteraction,
        value: &ResolvedValue<'_>,
    ) -> Result<Self, SlashArgError> {
        match *value {
            ResolvedValue::User(user, member) => Ok(Self {
                user: user.clone(),
                member: member.cloned(),
            }),
            _ => Err(SlashArgError::new_command_structure_mismatch("expected user"))
        }
    }

    fn create(builder: CreateCommandOption<'_>) -> CreateCommandOption<'_> {
        builder.kind(CommandOptionType::User)
    }
}

#[derive(Debug, Clone)]
pub struct SlashMember {
    pub user: User,
    pub member: PartialMember,
}

#[serenity::async_trait]
impl SlashArgument for SlashMember {
    async fn extract(
        _ctx: &serenity::gateway::client::Context,
        _interaction: &CommandInteraction,
        value: &ResolvedValue<'_>,
    ) -> Result<Self, SlashArgError> {
        match *value {
            ResolvedValue::User(user, Some(member)) => return Ok(Self {
                user: user.clone(),
                member: member.clone(),
            }),
            // delegate to this method to get the correct error
            _ => drop(<PartialMember as SlashArgument>::extract(_ctx, _interaction, value).await?)
        }

        // this is functionally unreachable
        Err(SlashArgError::new_command_structure_mismatch("expected member"))
    }

    fn create(builder: CreateCommandOption<'_>) -> CreateCommandOption<'_> {
        builder.kind(CommandOptionType::User)
    }
}

macro_rules! impl_shared_user_fn {
    ($($t:tt)*) => {
        impl SlashUser {
            $($t)*
        }
        impl SlashMember {
            $($t)*
        }
    };
}

impl_shared_user_fn! {
    pub fn display_name(&self) -> &str {
        self.member()
            .and_then(|m| m.nick.as_deref())
            .unwrap_or_else(|| self.user.display_name())
    }

    pub fn face(&self) -> String {
        // CMBK: PartialMember has no guild avatar field
        self.user.face()
    }

    pub fn from_resolved(ctx: HContext<'_>, user: User) -> anyhow::Result<Self> {
        let member = ctx.interaction.data
            .resolved.members
            .get(&user.id);

        Self::new_priv(user, member)
    }
}

impl SlashUser {
    fn new_priv(user: User, member: Option<&PartialMember>) -> anyhow::Result<Self> {
        Ok(Self {
            user,
            member: member.cloned(),
        })
    }

    fn member(&self) -> Option<&PartialMember> {
        self.member.as_ref()
    }
}

impl SlashMember {
    fn new_priv(user: User, member: Option<&PartialMember>) -> anyhow::Result<Self> {
        let member = member.context("expected member")?;
        Ok(Self {
            user,
            member: member.clone(),
        })
    }

    fn member(&self) -> Option<&PartialMember> {
        Some(&self.member)
    }
}

impl From<SlashMember> for SlashUser {
    fn from(value: SlashMember) -> Self {
        Self {
            user: value.user,
            member: Some(value.member),
        }
    }
}

impl Mentionable for SlashUser {
    fn mention(&self) -> Mention {
        Mention::User(self.user.id)
    }
}

impl Mentionable for SlashMember {
    fn mention(&self) -> Mention {
        Mention::User(self.user.id)
    }
}
