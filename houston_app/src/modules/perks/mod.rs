use bson_model::{Filter, ModelDocument as _};
use chrono::TimeDelta;
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
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES
    }

    fn commands(&self, config: &HBotConfig) -> impl IntoIterator<Item = Command> {
        let perks = config.perks().unwrap();
        let mut c = vec![
            slashies::perk_admin::perk_admin(perks),
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

    fn buttons(&self, config: &HBotConfig) -> impl IntoIterator<Item = ButtonAction> {
        let perks = config.perks().unwrap();
        let mut b = vec![buttons::shop::View::ACTION];

        if perks.birthday.is_some() {
            b.push(buttons::birthday::Set::ACTION);
        }

        b
    }

    fn validate(&self, config: &HBotConfig) -> Result {
        let perks = config.perks().unwrap();

        anyhow::ensure!(
            config.mongodb_uri.is_some(),
            "`perks` requires setting `mongodb_uri`"
        );

        if let Some(birthday) = &perks.birthday {
            anyhow::ensure!(
                u16::try_from(birthday.regions.len()).is_ok(),
                "can only specify up to {} birthday regions",
                u16::MAX
            );
        }

        log::info!("Perks are enabled.");

        if perks.rainbow.is_some() && perks.check_interval < TimeDelta::minutes(2) {
            log::warn!(
                "`perks.check_interval` is less than 2 minutes and rainbow role is enabled. \
                 You will likely hit Discord rate limits with this configuration. \
                 Increase `perks.check_interval` to at least 2 minutes."
            );
        }

        Ok(())
    }

    async fn db_init(self, data: Arc<HBotData>, db: mongodb::Database) -> Result {
        let perks = data.config().perks().unwrap();

        model::Wallet::update_indices(&db).await?;
        model::ActivePerk::update_indices(&db).await?;

        if perks.role_edit.is_some() {
            model::UniqueRole::update_indices(&db).await?;
        }

        if perks.birthday.is_some() {
            model::Birthday::update_indices(&db).await?;
        }

        Ok(())
    }

    fn event_handler(self) -> Option<Box<dyn EventHandler>> {
        Some(Box::new(self))
    }
}

super::impl_handler!(Module, |_, ctx| match _ {
    FullEvent::InteractionCreate { .. }
    | FullEvent::Message { .. }
    | FullEvent::ReactionAdd { .. } => check_perks(ctx),
});

async fn check_perks(ctx: &Context) {
    if let Err(why) = check_perks_inner(ctx).await {
        log::error!("Perk check failed: {why:?}");
    }
}

async fn check_perks_inner(ctx: &Context) -> Result {
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

    // handle updates and expiry in parallel.
    // don't use `try_join!` here since we want to run others to
    // completion even if one of them ends up failing for any reason.
    let (result, ()) = tokio::join!(check_expiry(ctx, now), update_perks(ctx, now));
    result
}

async fn update_perks(ctx: &Context, now: DateTime<Utc>) {
    for kind in effects::Effect::all() {
        if let Err(why) = kind.update(ctx, now).await {
            log::error!("Failed update for perk effect {kind:?}: {why:?}");
        }
    }
}

async fn check_expiry(ctx: &Context, now: DateTime<Utc>) -> Result {
    let data = ctx.data_ref::<HContextData>();
    let db = data.database()?;

    let filter = model::ActivePerk::filter()
        .until(Filter::Lt(now))
        .into_document()?;

    let mut query = model::ActivePerk::collection(db)
        .find(filter)
        .await
        .context("failed to begin expired perk query")?;

    while let Some(perk) = query.next().await {
        let perk = perk.context("failed to get next expired perk")?;

        log::debug!(
            "Trying to disable perk {:?} for {} in {}.",
            perk.effect,
            perk.user,
            perk.guild
        );

        // size of the `disable` future blows up the size `check_perks`
        // so it is boxed here since it's also rarely reached
        let args = effects::Args::new(ctx, perk.guild, perk.user);
        Box::pin(perk.effect.disable(args)).await?;

        model::ActivePerk::collection(db)
            .delete_one(perk.self_filter())
            .await
            .context("failed to delete expired perk")?;
    }

    Ok(())
}
