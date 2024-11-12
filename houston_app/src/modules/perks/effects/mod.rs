use serenity::gateway::client::Context;

use crate::prelude::*;

pub mod rainbow_role;

pub struct Args<'a> {
    pub ctx: &'a Context,
    pub guild_id: GuildId,
    pub user_id: UserId,
}

pub trait Effect {
    async fn enable(&self, args: Args<'_>) -> HResult;
    async fn disable(&self, args: Args<'_>) -> HResult;

    async fn update(&self, ctx: &Context) -> HResult {
        _ = ctx;
        Ok(())
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq,
    serde::Serialize, serde::Deserialize, poise::ChoiceParameter,
)]
pub enum Kind {
    RainbowRole,
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

impl Kind {
    impl_kind_fn!(enable => args: Args<'_>);
    impl_kind_fn!(disable => args: Args<'_>);
    impl_kind_fn!(update => args: &Context);

    pub fn all() -> &'static [Self] {
        &[
            Self::RainbowRole,
        ]
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::RainbowRole => "Rainbow Role",
        }
    }
}
