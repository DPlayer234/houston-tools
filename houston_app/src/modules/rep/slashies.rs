use std::pin::pin;
use std::slice;
use std::time::Duration;

use bson_model::Filter;
use chrono::{DateTime, Utc};
use mongodb::options::ReturnDocument;
use rand::prelude::*;
use tokio::time::timeout;

use super::model;
use crate::fmt::discord::TimeMentionable as _;
use crate::modules::Module as _;
use crate::slashies::prelude::*;

/// We try to query the DB for the cooldown first, so we only defer with
/// non-ephemeral messages when it is likely that the user isn't on cooldown.
///
/// However, we don't want to fail the command entirely if the DB takes too long
/// to respond, so we degrade the response display a bit instead.
///
/// ... Now, if the ping to Discord is also too slow, that's now a bigger
/// problem, but I guess we just fail the real defer then.
const TOO_LONG: Duration = Duration::from_millis(1500);

/// Gives a reputation point to another server member.
#[context_command(user, name = "Rep+", contexts = "Guild", integration_types = "Guild")]
pub async fn rep_context(ctx: Context<'_>, member: SlashMember<'_>) -> Result {
    rep_core(ctx, member).await
}

/// Gives a reputation point to another server member.
#[chat_command(contexts = "Guild", integration_types = "Guild")]
pub async fn rep(
    ctx: Context<'_>,
    /// The server member to rep.
    member: SlashMember<'_>,
) -> Result {
    rep_core(ctx, member).await
}

async fn rep_core(ctx: Context<'_>, member: SlashMember<'_>) -> Result {
    anyhow::ensure!(
        member.user.id != ctx.user().id,
        HArgError::new("How lonely are you to rep yourself?")
    );

    anyhow::ensure!(
        !member.user.bot(),
        HArgError::new("Uh, that's a bot. Maybe pick someone real.")
    );

    let data = ctx.data_ref();
    let db = data.database()?;
    let rep = data.config().rep()?;
    let guild_id = ctx.require_guild_id()?;

    let cooldown_check = async {
        // fetch/upsert the user's base state.
        // we need this to ensure the document exists for the update.
        let filter = model::Record::filter()
            .user(ctx.user().id)
            .guild(guild_id)
            .into_document()?;

        let upsert = model::Record::update()
            .set_on_insert(|r| r.cooldown_ends(DateTime::UNIX_EPOCH))
            .into_document()?;

        let self_state = model::Record::collection(db)
            .find_one_and_update(filter, upsert)
            .upsert(true)
            .return_document(ReturnDocument::After)
            .await?
            .context("got nothing on upsert")?;

        let now = Utc::now();

        // preliminary cooldown check
        if self_state.cooldown_ends > now {
            return bail_on_cooldown(&self_state).await;
        }

        let next_cooldown_end = Utc::now()
            .checked_add_signed(rep.cooldown)
            .context("cooldown broke the end of time")?;

        // try to update the cooldown in the document
        let filter = model::Record::filter()
            .user(ctx.user().id)
            .guild(guild_id)
            .cooldown_ends(Filter::Lte(now))
            .into_document()?;

        let update = model::Record::update()
            .set(|r| r.cooldown_ends(next_cooldown_end))
            .into_document()?;

        let is_updated = model::Record::collection(db)
            .find_one_and_update(filter, update)
            .await?
            .is_some();

        // this will ensure that concurrent commands by the same user don't pass.
        // that _shouldn't_ be possible due to rate limits and such but... yeah.
        // if the update fails, that means the filter didn't match.
        if !is_updated {
            return bail_on_cooldown(&self_state).await;
        }

        Ok(())
    };

    // for better UX, try not to show the cooldown error to everyone in most cases.
    // when that doesn't work out, also fine, but usually the db isn't that slow.
    // ... except we also don't want to defer in the successful case because edits
    // can't trigger notifications. so don't defer at all if possible.
    let res = match defer_if_too_long(ctx, cooldown_check).await {
        Ok(inner) => {
            // evaluate `db_op` result.
            inner?;

            // send the message as soon as we're sure it's correct
            let emoji = EMOJIS.choose(&mut rand::rng()).expect("EMOJIS not empty");
            let content = format!(
                "{emoji} | {} has given {} a reputation point!",
                ctx.user().mention(),
                member.mention(),
            );

            let allowed_mentions =
                CreateAllowedMentions::new().users(slice::from_ref(&member.user.id));

            let reply = CreateReply::new()
                .content(content)
                .allowed_mentions(allowed_mentions);

            // low chance this fails. propagate the error to the `if let` below.
            ctx.send(reply).await
        },
        Err(why) => Err(why),
    };

    // DON'T propagate this out. this error may indicate that deferring failed, plus
    // db operations have already happened, so we want to finish those even if this
    // fails. and to send to discord isn't unreasonable because we introduce delay.
    if let Err(why) = res {
        log::error!(
            "Failed to send `/rep` confirm: in {guild_id}, by {}, to {}. {why:?}",
            ctx.user().name,
            member.user.name,
        );
    }

    // actually rep the target user
    let filter = model::Record::filter()
        .user(member.user.id)
        .guild(guild_id)
        .into_document()?;

    let update = model::Record::update()
        .set_on_insert(|r| r.cooldown_ends(DateTime::UNIX_EPOCH))
        .inc(|r| r.received(1))
        .into_document()?;

    model::Record::collection(db)
        .update_one(filter, update)
        .upsert(true)
        .await?;

    log::trace!("{} repped {}.", ctx.user().name, member.user.name);

    if rep.cash_gain != 0 && crate::modules::perks::Module.enabled(data.config()) {
        use crate::modules::perks::Item;
        use crate::modules::perks::model::{Wallet, WalletExt as _};

        let amount: i64 = rep.cash_gain.into();

        Wallet::collection(db)
            .add_items(guild_id, member.user.id, &[(Item::Cash, amount)])
            .await?;

        log::trace!("{} gained {} cash.", member.user.name, amount);
    }

    Ok(())
}

async fn bail_on_cooldown(self_state: &model::Record) -> Result {
    let time = self_state.cooldown_ends.short_date_time();
    Err(HArgError::new(format!("Nope. You can rep again at: {time}")).into())
}

/// Defers via the `ctx` if `fut` takes [`TOO_LONG`] to finish.
///
/// The outer result represents the defer result or [`Ok`] if it didn't defer.
async fn defer_if_too_long<F>(ctx: Context<'_>, fut: F) -> serenity::Result<F::Output>
where
    F: Future,
{
    let mut fut = pin!(fut);
    match timeout(TOO_LONG, &mut fut).await {
        Ok(ok) => Ok(ok),
        Err(_) => {
            ctx.defer(false).await?;
            Ok(fut.await)
        },
    }
}

const EMOJIS: &[&str] = &[
    "🐶",
    "🐱",
    "🐭",
    "🐹",
    "🐰",
    "🦊",
    "🐻",
    "🐼",
    "🐻‍❄️",
    "🐨",
    "🐯",
    "🦁",
    "🐮",
    "🐷",
    "🐸",
    "🐔",
    "🐧",
    "🐦",
    "🪿",
    "🦆",
    "🦅",
    "🦉",
    "🦇",
    "🐺",
    "🐗",
    "🐣",
];
