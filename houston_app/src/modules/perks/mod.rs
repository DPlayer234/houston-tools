use std::sync::Arc;

use bson_model::Filter;
use chrono::prelude::*;

use super::prelude::*;

pub mod buttons;
pub mod config;
mod day_of_year;
mod effects;
mod items;
pub mod model;
mod slashies;

pub use config::Config;
pub use day_of_year::DayOfYear;
pub use items::Item;

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &HBotConfig) -> bool {
        config.perks.is_some()
    }

    fn intents(&self, _config: &HBotConfig) -> GatewayIntents {
        GatewayIntents::GUILD_MESSAGES
    }

    fn commands(&self, config: &HBotConfig) -> impl IntoIterator<Item = Command> {
        let perks = config.perks().unwrap();
        let mut c = vec![
            slashies::perk_admin::perk_admin(),
            slashies::shop::shop(),
            slashies::wallet::wallet(),
        ];

        if let Some(pushpin) = &perks.pushpin {
            let mut pin = slashies::pushpin::pushpin_pin();
            let mut unpin = slashies::pushpin::pushpin_unpin();
            pin.data.name = format!("Use {}: Pin", pushpin.name).into();
            unpin.data.name = format!("Use {}: Unpin", pushpin.name).into();

            c.extend([pin, unpin]);
        }

        if let Some(role_edit) = &perks.role_edit {
            let mut edit = slashies::role_edit::role_edit();
            edit.data.description =
                format!("Use {}: Edit your unique role.", role_edit.name).into();

            c.push(edit);
        }

        if perks.birthday.is_some() {
            c.push(slashies::birthday::birthday());
        }

        c
    }

    fn validate(&self, config: &HBotConfig) -> Result {
        anyhow::ensure!(config.mongodb_uri.is_some(), "perks requires a mongodb_uri");

        let perks = config.perks().unwrap();
        log::info!("Perks are enabled.");

        if let Some(r) = &perks.rainbow {
            log::trace!("Rainbow Role is enabled: {} guild(s)", r.guilds.len());
        }

        Ok(())
    }

    async fn db_init(self, _data: Arc<HBotData>, db: mongodb::Database) -> Result {
        use model::*;

        use crate::helper::bson::update_indices;
        update_indices(Wallet::collection(&db), Wallet::indices()).await?;
        update_indices(ActivePerk::collection(&db), ActivePerk::indices()).await?;
        update_indices(UniqueRole::collection(&db), UniqueRole::indices()).await?;
        update_indices(Birthday::collection(&db), Birthday::indices()).await?;
        Ok(())
    }
}

pub fn dispatch_check_perks(ctx: &Context) {
    let data = ctx.data_ref::<HContextData>();
    if Module.enabled(data.config()) {
        tokio::task::spawn(check_perks_impl(ctx.clone()));
    }
}

async fn check_perks_impl(ctx: Context) {
    if let Err(why) = check_perks_core(ctx).await {
        log::error!("Perk check failed: {why:?}");
    }
}

async fn check_perks_core(ctx: Context) -> Result {
    let data = ctx.data_ref::<HContextData>();
    let perks = data.config().perks()?;
    let last = *perks.last_check.read().await;
    let next = last
        .checked_add_signed(perks.check_interval)
        .context("time has broken")?;

    let now = Utc::now();
    if now < next {
        // no need to check yet
        return Ok(());
    }

    // we hold this lock for the entire process so we can avoid others racing within
    // this method. exit here on error since it means multiple threads got past
    // the `read` above and hit this, but that's fine
    let Ok(mut last_check) = perks.last_check.try_write() else {
        return Ok(());
    };

    *last_check = now;

    // handle updates to the effects in parallel
    tokio::spawn({
        let ctx = ctx.clone();
        async move {
            for kind in effects::Effect::all() {
                if let Err(why) = kind.update(&ctx, now).await {
                    log::error!("Failed update for perk effect {kind:?}: {why:?}");
                }
            }
        }
    });

    let db = data.database()?;

    // search for expiring perks
    let filter = model::ActivePerk::filter()
        .until(Filter::Lt(now))
        .into_document()?;

    let mut query = model::ActivePerk::collection(db).find(filter).await?;

    while let Some(perk) = query.try_next().await? {
        let args = effects::Args::new(&ctx, perk.guild, perk.user);
        perk.effect.disable(args).await?;

        model::ActivePerk::collection(db)
            .delete_one(perk.self_filter())
            .await?;
    }

    Ok(())
}
