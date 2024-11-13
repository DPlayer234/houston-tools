use serenity::gateway::client::Context;

use super::config::{Config, EffectPrice};
use crate::prelude::*;

pub mod rainbow_role;

#[derive(Debug, Clone)]
pub struct Args<'a> {
    pub ctx: &'a Context,
    pub guild_id: GuildId,
    pub user_id: UserId,
}

trait Shape {
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
pub struct EffectInfo {
    pub name: &'static str,
    pub description: &'static str,
}

macro_rules! impl_kind_fn {
    ($name:ident => $args:ident: $args_ty:ty) => {
        pub async fn $name(self, $args: $args_ty) -> HResult {
            match self {
                Self::RainbowRole => rainbow_role::RainbowRole.$name($args).await,
            }
        }
    };
}

impl Effect {
    impl_kind_fn!(enable => args: Args<'_>);
    impl_kind_fn!(disable => args: Args<'_>);
    impl_kind_fn!(update => args: &Context);

    pub fn all() -> &'static [Self] {
        &[
            Self::RainbowRole,
        ]
    }

    pub fn info(self) -> EffectInfo {
        match self {
            Self::RainbowRole => EffectInfo { name: "Rainbow Role", description: "A role with regularly changing color." },
        }
    }

    pub fn price(self, perks: &Config) -> Option<EffectPrice> {
        match self {
            Self::RainbowRole => perks.rainbow.as_ref().map(|r| r.price),
        }
    }
}
