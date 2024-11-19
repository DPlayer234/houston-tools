use anyhow::Context as _;
use bson::{doc, Bson};
use chrono::prelude::*;
use chrono::TimeDelta;
use serenity::futures::TryStreamExt;
use serenity::prelude::*;
use tokio::sync::RwLock;

use super::Module as _;
use crate::config::HBotConfig;
use crate::helper::bson::doc_object_id;
use crate::prelude::*;

// 2 minutes is about the minimum safe interval for constant role updates
// we go a little higher since we use this interval for other stuff too
const CHECK_INTERVAL: TimeDelta = TimeDelta::minutes(3);

pub mod buttons;
pub mod config;
mod effects;
mod items;
pub mod model;
mod slashies;

pub use config::Config;
pub use items::Item;

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &HBotConfig) -> bool {
        config.perks.is_some()
    }

    fn intents(&self, _config: &HBotConfig) -> GatewayIntents {
        GatewayIntents::GUILD_MESSAGES
    }

    fn commands(&self, config: &HBotConfig) -> impl IntoIterator<Item = HCommand> {
        let perks = config.perks().unwrap();
        let mut c = vec![
            slashies::perk_admin::perk_admin(),
            slashies::shop::shop(),
            slashies::wallet::wallet(),
        ];

        if let Some(pushpin) = &perks.pushpin {
            let mut pin = slashies::pushpin::pushpin_pin();
            let mut unpin = slashies::pushpin::pushpin_unpin();
            pin.context_menu_name = Some(format!("Use {}: Pin", pushpin.name).into());
            unpin.context_menu_name = Some(format!("Use {}: Unpin", pushpin.name).into());

            c.extend([pin, unpin]);
        }

        if let Some(role_edit) = &perks.role_edit {
            let mut edit = slashies::role_edit::role_edit();
            edit.description = Some(format!("Use {}: Edit your custom role.", role_edit.name).into());

            c.push(edit);
        }

        c
    }

    fn db_init(db: &mongodb::Database) -> mongodb::BoxFuture<'_, HResult> {
        Box::pin(async move {
            model::Wallet::collection(db).create_indexes(model::Wallet::indices()).await?;
            model::ActivePerk::collection(db).create_indexes(model::ActivePerk::indices()).await?;
            model::UniqueRole::collection(db).create_indexes(model::UniqueRole::indices()).await?;
            Ok(())
        })
    }

    fn validate(&self, config: &HBotConfig) -> HResult {
        if config.mongodb_uri.is_none() {
            anyhow::bail!("perks requires a mongodb_uri");
        }

        let perks = config.perks().unwrap();
        log::info!("Perks are enabled.");

        if let Some(r) = &perks.rainbow {
            log::trace!("Rainbow Role is enabled: {} guild(s)", r.guilds.len());
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct PerkState {
    last_check: RwLock<DateTime<Utc>>,
}

pub fn dispatch_check_perks(ctx: &Context) {
    let data = ctx.data_ref::<HFrameworkData>();
    if Module.enabled(data.config()) {
        tokio::task::spawn(check_perks_impl(ctx.clone()));
    }
}

async fn check_perks_impl(ctx: Context) {
    if let Err(why) = check_perks_core(ctx).await {
        log::error!("Perk check failed: {why:?}");
    }
}

async fn check_perks_core(ctx: Context) -> HResult {
    let data = ctx.data_ref::<HFrameworkData>();
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
            for kind in effects::Effect::all() {
                if let Err(why) = kind.update(&ctx).await {
                    log::error!("Failed update for perk effect {kind:?}: {why:?}");
                }
            }
        }
    });

    let db = data.database()?;

    // search for expiring perks
    let filter = doc! {
        "until": {
            "$lt": Bson::DateTime(now.into()),
        },
    };

    let mut query = model::ActivePerk::collection(db)
        .find(filter)
        .await?;

    while let Some(perk) = query.try_next().await? {
        let args = effects::Args::new(&ctx, perk.guild, perk.user);
        perk.effect.disable(args).await?;

        model::ActivePerk::collection(db)
            .delete_one(doc_object_id!(perk))
            .await?;
    }

    Ok(())
}
