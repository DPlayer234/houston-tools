use std::pin::pin;
use std::slice;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use bson_model::Filter;
use chrono::{DateTime, Utc};
use rand::prelude::*;
use tokio::time::timeout;

use super::model;
use crate::fmt::discord::TimeMentionable as _;
use crate::helper::bson::is_upsert_duplicate_key;
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

// this code has a couple of frankly weird conditions:
// - we don't want to defer too early in the failure case so the cooldown error
//   isn't shown to everyone.
// - we don't want to defer _at all_ in the success case so the rep message is
//   the initial message, which is needed to actually trigger a notification.
// - we can't wait too long to defer and/or send because that would time out the
//   command interaction.
// - there are at least 3 required database operations.
// so consider that when touching this.
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

    // defer appropriately when the initial check takes too long
    let ephemeral = AtomicBool::new(false);
    let defer = async { ctx.defer(ephemeral.load(Ordering::Relaxed)).await };

    let cooldown_check = async {
        let now = Utc::now();
        let next_cooldown_end = Utc::now()
            .checked_add_signed(rep.cooldown)
            .context("cooldown broke the end of time")?;

        // try to update the cooldown in the document.
        // this is performed as an upsert with a unique index over [user, guild].
        // if this fails due to a `DuplicateKey`, this means that the document exists
        // and its cooldown hasn't expired yet.
        let filter = model::Record::filter()
            .user(ctx.user().id)
            .guild(guild_id)
            .cooldown_ends(Filter::Lte(now))
            .into_document()?;

        let update = model::Record::update()
            .set(|r| r.cooldown_ends(next_cooldown_end))
            .into_document()?;

        let update_res = model::Record::collection(db)
            .update_one(filter, update)
            .upsert(true)
            .await;

        let on_cooldown = match update_res {
            Ok(_) => false,
            Err(why) if is_upsert_duplicate_key(&why) => true,
            Err(why) => anyhow::bail!(why),
        };

        if on_cooldown {
            // set it to defer ephemerally if on cooldown
            ephemeral.store(true, Ordering::Relaxed);

            // the only downside to the upsert-or-fail approach:
            // needs 1 more query for the failure case.
            // i'd say it's worth 1 less query for the success case.
            return throw_cooldown_error(ctx).await;
        }

        Ok(())
    };

    // for better UX, try not to show the cooldown error to everyone in most cases.
    // when that doesn't work out, also fine, but usually the db isn't that slow.
    // ... except we also don't want to defer in the successful case because edits
    // can't trigger notifications. so don't defer at all if possible.
    let (cooldown_check, defer) = if_too_long(cooldown_check, defer).await;

    // evaluate cooldown result
    cooldown_check?;

    // if the defer part ran and failed, don't try to send a message
    // propagate the error from both branches to the logging below
    let res = if let Some(Err(why)) = defer {
        Err(why)
    } else {
        // send the message as soon as we're sure it's correct
        let emoji = EMOJIS.choose(&mut rand::rng()).expect("EMOJIS not empty");
        let content = format!(
            "{emoji} | {} has given {} a reputation point!",
            ctx.user().mention(),
            member.mention(),
        );

        let allowed_mentions = CreateAllowedMentions::new().users(slice::from_ref(&member.user.id));

        let reply = CreateReply::new()
            .content(content)
            .allowed_mentions(allowed_mentions);

        ctx.send(reply).await
    };

    // DON'T propagate this out. this error may indicate that deferring failed, plus
    // db operations have already happened, so we want to finish those even if this
    // fails. and failing to send to discord isn't unreasonable due to the delay.
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

async fn throw_cooldown_error(ctx: Context<'_>) -> Result {
    let data = ctx.data_ref();
    let db = data.database()?;
    let guild_id = ctx.require_guild_id()?;

    let filter = model::Record::filter()
        .user(ctx.user().id)
        .guild(guild_id)
        .into_document()?;

    let self_state = model::Record::collection(db)
        .find_one(filter)
        .await?
        .context("got nothing after upsert")?;

    let time = self_state.cooldown_ends.short_date_time();
    Err(HArgError::new(format!("Nope. You can rep again at: {time}")).into())
}

async fn if_too_long<F, I>(fut: F, intercept: I) -> (F::Output, Option<I::Output>)
where
    F: Future,
    I: Future,
{
    let mut fut = pin!(fut);
    match timeout(TOO_LONG, &mut fut).await {
        Ok(f) => (f, None),
        Err(_) => tokio::join!(fut, async { Some(intercept.await) }),
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
