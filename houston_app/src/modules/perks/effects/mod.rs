use serenity::gateway::client::Context;

use super::config::{Config, EffectPrice};
use crate::prelude::*;

pub mod rainbow_role;

#[derive(Debug, Clone, Copy)]
pub struct Args<'a> {
    pub ctx: &'a Context,
    pub guild_id: GuildId,
    pub user_id: UserId,
}

impl<'a> Args<'a> {
    pub fn new(ctx: &'a Context, guild_id: GuildId, user_id: UserId) -> Self {
        Self { ctx, guild_id, user_id }
    }
}

trait Shape {
    async fn supported(&self, args: Args<'_>) -> anyhow::Result<bool> {
        _ = args;
        Ok(true)
    }

    async fn enable(&self, args: Args<'_>) -> HResult {
        _ = args;
        Ok(())
    }

    async fn disable(&self, args: Args<'_>) -> HResult {
        _ = args;
        Ok(())
    }

    async fn update(&self, ctx: &Context) -> HResult {
        _ = ctx;
        Ok(())
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq,
    serde::Serialize, serde::Deserialize, poise::ChoiceParameter,
)]
pub enum Effect {
    RainbowRole,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EffectInfo<'a> {
    pub name: &'a str,
    pub description: &'a str,
}

macro_rules! impl_kind_fn {
    ($name:ident ( $args:ident: $args_ty:ty ) -> $ret:ty) => {
        pub async fn $name(self, $args: $args_ty) -> $ret {
            match self {
                Self::RainbowRole => rainbow_role::RainbowRole.$name($args).await,
            }
        }
    };
}

impl Effect {
    impl_kind_fn!(supported(args: Args<'_>) -> anyhow::Result<bool>);
    impl_kind_fn!(enable(args: Args<'_>) -> HResult);
    impl_kind_fn!(disable(args: Args<'_>) -> HResult);
    impl_kind_fn!(update(args: &Context) -> HResult);

    pub fn all() -> &'static [Self] {
        &[
            Self::RainbowRole,
        ]
    }

    pub fn info(self, perks: &Config) -> EffectInfo<'_> {
        match self {
            Self::RainbowRole => perks.rainbow.as_ref()
                .map(|r| EffectInfo { name: &r.name, description: &r.description })
                .unwrap_or(const { EffectInfo { name: "Rainbow Role", description: "A role with regularly changing color." } }),
        }
    }

    pub fn price(self, perks: &Config) -> Option<EffectPrice> {
        match self {
            Self::RainbowRole => perks.rainbow.as_ref().map(|r| r.price),
        }
    }
}
