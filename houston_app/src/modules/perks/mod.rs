use chrono::prelude::*;
use serenity::prelude::*;
use tokio::sync::RwLock;

use crate::prelude::*;

pub mod buttons;
#[cfg(feature = "db")]
pub mod model;
#[cfg(feature = "db")]
mod effects;
#[cfg(feature = "db")]
mod slashies;

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &super::config::HBotConfig) -> bool {
        config.perks.is_some()
    }

    fn intents(&self) -> GatewayIntents {
        GatewayIntents::GUILD_MESSAGES
    }

    fn commands(&self) -> impl IntoIterator<Item = HCommand> {
        [
            #[cfg(feature = "db")]
            slashies::perk_admin::perk_admin(),
            #[cfg(feature = "db")]
            slashies::perk_store::perk_store(),
        ]
    }

    #[cfg(feature = "db")]
    fn db_init(db: &mongodb::Database) -> mongodb::BoxFuture<'_, HResult> {
        Box::pin(async move {
            model::Wallet::collection(db).create_indexes(model::Wallet::indices()).await?;
            model::ActivePerk::collection(db).create_indexes(model::ActivePerk::indices()).await?;
            Ok(())
        })
    }

    fn validate(&self, config: &config::HBotConfig) -> HResult {
        if config.mongodb_uri.is_none() {
            anyhow::bail!("perks requires a mongodb_uri");
        }

        let perks = config.perks.as_ref().expect("must be enabled");
        log::info!("Perks are enabled.");

        if let Some(r) = &perks.rainbow {
            log::trace!("Rainbow Role is enabled: {} role(s)", r.role.len());
        }

        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    _enable: Option<bool>,
    pub rainbow: Option<RainbowConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StoreConfig {
    pub cost: u32,
    pub duration: u32,
}

#[derive(Debug, serde::Deserialize)]
pub struct RainbowConfig {
    #[serde(flatten)]
    pub store: StoreConfig,
    pub role: Vec<RainbowRoleEntry>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RainbowRoleEntry {
    pub guild: GuildId,
    pub role: RoleId,
}

#[derive(Debug, Default)]
pub struct PerkState {
    last_check: RwLock<DateTime<Utc>>,
}

#[cfg_attr(not(feature = "db"), expect(unused_variables))]
pub async fn check_perks(ctx: Context) {
    #[cfg(feature = "db")]
    if let Err(why) = check_perks_core(ctx).await {
        log::error!("Perk check failed: {why:?}");
    }
}

#[cfg(feature = "db")]
async fn check_perks_core(ctx: Context) -> HResult {
    const CHECK_INTERVAL: TimeDelta = TimeDelta::minutes(5);

    use anyhow::Context;
    use bson::{doc, Bson};
    use chrono::TimeDelta;
    use serenity::futures::TryStreamExt;

    use crate::modules::Module;

    let data = ctx.data_ref::<HBotData>();
    if !Module.enabled(data.config()) {
        return Ok(());
    }

    let state = data.perk_state();
    let last = *state.last_check.read().await;
    let next = last
        .checked_add_signed(CHECK_INTERVAL)
        .context("time has broken")?;

    let now = Utc::now();
    if now < next {
        // no need to check yet
        return Ok(());
    }

    // we hold this lock for the entire process
    // so we can avoid others racing within this method
    let mut last_check = state.last_check.try_write()?;
    *last_check = now;

    // handle updates to the effects in parallel
    tokio::spawn({
        let ctx = ctx.clone();
        async move {
            for kind in effects::Kind::all() {
                if let Err(why) = kind.update(&ctx).await {
                    log::error!("Failed update for perk effect {kind:?}: {why:?}");
                }
            }
        }
    });

    let db = data.database()?;

    // search for expiring perks
    {
        let filter = doc! {
            "until": {
                "$lt": Bson::DateTime(now.into()),
            },
        };

        let mut query = model::ActivePerk::collection(db)
            .find(filter)
            .await?;

        while let Some(perk) = query.try_next().await? {
            let args = effects::Args {
                ctx: &ctx,
                guild_id: perk.guild,
                user_id: perk.user,
            };

            perk.effect.disable(args).await?;

            #[allow(clippy::used_underscore_binding)]
            let filter = doc! {
                "_id": perk._id,
            };

            model::ActivePerk::collection(db)
                .delete_one(filter)
                .await?;
        }
    }

    Ok(())
}
