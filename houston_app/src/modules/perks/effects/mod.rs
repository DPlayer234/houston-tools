use bson::Bson;
use chrono::{DateTime, Utc};

use super::config::{Config, EffectPrice};
use crate::modules::prelude::*;

mod birthday;
mod rainbow_role;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, houston_cmd::ChoiceArg,
)]
pub enum Effect {
    RainbowRole,
    Birthday,
}

#[derive(Debug, Clone, Copy)]
pub struct Args<'a> {
    pub ctx: &'a Context,
    pub guild_id: GuildId,
    pub user_id: UserId,
}

impl<'a> Args<'a> {
    pub fn new(ctx: &'a Context, guild_id: GuildId, user_id: UserId) -> Self {
        Self {
            ctx,
            guild_id,
            user_id,
        }
    }
}

trait Shape {
    async fn supported(&self, args: Args<'_>) -> Result<bool> {
        _ = args;
        Ok(true)
    }

    async fn enable(&self, args: Args<'_>, state: Option<Bson>) -> Result {
        _ = args;
        _ = state;
        Ok(())
    }

    async fn disable(&self, args: Args<'_>) -> Result {
        _ = args;
        Ok(())
    }

    async fn update(&self, ctx: &Context, now: DateTime<Utc>) -> Result {
        _ = ctx;
        _ = now;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EffectInfo<'a> {
    pub name: &'a str,
    pub description: &'a str,
}

macro_rules! impl_kind_fn {
    ($name:ident ( $($args:ident: $args_ty:ty),* ) -> $ret:ty) => {
        pub async fn $name(self, $($args: $args_ty),*) -> $ret {
            match self {
                Self::RainbowRole => rainbow_role::RainbowRole.$name($($args),*).await,
                Self::Birthday => birthday::Birthday.$name($($args),*).await,
            }
        }
    };
}

impl Effect {
    impl_kind_fn!(supported(args: Args<'_>) -> Result<bool>);
    impl_kind_fn!(enable(args: Args<'_>, state: Option<Bson>) -> Result);
    impl_kind_fn!(disable(args: Args<'_>) -> Result);
    impl_kind_fn!(update(args: &Context, now: DateTime<Utc>) -> Result);

    pub fn all() -> &'static [Self] {
        &[Self::RainbowRole, Self::Birthday]
    }

    pub fn info(self, perks: &Config) -> EffectInfo<'_> {
        const UNSET: EffectInfo<'_> = EffectInfo {
            name: "Unset Effect",
            description: "Effect is not configured.",
        };

        match self {
            Self::RainbowRole => perks
                .rainbow
                .as_ref()
                .map(|r| EffectInfo {
                    name: &r.name,
                    description: &r.description,
                })
                .unwrap_or(UNSET),
            Self::Birthday => EffectInfo {
                name: "Birthday Haver",
                description: "Party time.",
            },
        }
    }

    pub fn price(self, perks: &Config) -> Option<EffectPrice> {
        match self {
            Self::RainbowRole => perks.rainbow.as_ref().map(|r| r.price),
            Self::Birthday => None,
        }
    }
}

/// Checks for certain allowed Discord error codes, which will be turned into
/// [`Ok(None)`](Ok). Other errors are passed through as is.
///
/// This allows swallowing a couple errors you expect to run into.
///
/// Currently, the following error codes are okayed:
/// - 10007 (Unknown Member)
/// - 10013 (Unknown User)
///
/// If it was [`Ok`] to begin with, returns that value wrapped in [`Some`].
fn ok_allowed_discord_error<T>(
    result: Result<T, serenity::Error>,
) -> Result<Option<T>, serenity::Error> {
    use serenity::http::{HttpError, JsonErrorCode as J};

    if let Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(why))) = &result {
        if matches!(why.error.code, J::UnknownMember | J::UnknownUser) {
            return Ok(None);
        }
    }

    result.map(Some)
}

/// Checks whether a result contains a [`serenity::Error`] with the Discord
/// error code 10007 (Unknown Member).
///
/// If it does, returns [`Ok(false)`](Ok). Otherwise returns [`Ok(true)`](Ok)
/// for an [`Ok`] value or the original error.
fn is_known_member(result: Result) -> Result<bool> {
    use serenity::http::{HttpError, JsonErrorCode as J};

    if let Err(why) = &result {
        let why = why.downcast_ref();
        if let Some(serenity::Error::Http(HttpError::UnsuccessfulRequest(why))) = why {
            if matches!(why.error.code, J::UnknownMember | J::UnknownUser) {
                return Ok(false);
            }
        }
    }

    result.map(|_| true)
}
