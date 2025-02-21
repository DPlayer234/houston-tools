use std::char;
use std::sync::Arc;

use bson_model::{Filter, ModelDocument as _};
use mongodb::options::ReturnDocument;
use rand::prelude::*;
use utils::text::write_str::*;

use super::prelude::*;
use crate::fmt::discord::MessageLink;
use crate::fmt::replace_holes;
use crate::helper::is_unique_set;

pub mod buttons;
pub mod config;
pub mod model;
mod slashies;

pub use config::{BoardId, Config};

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &HBotConfig) -> bool {
        !config.starboard.is_empty()
    }

    fn intents(&self, _config: &HBotConfig) -> GatewayIntents {
        GatewayIntents::GUILD_MESSAGE_REACTIONS | GatewayIntents::GUILD_MESSAGES
    }

    fn commands(&self, _config: &HBotConfig) -> impl IntoIterator<Item = Command> {
        [slashies::starboard()]
    }

    fn validate(&self, config: &HBotConfig) -> Result {
        anyhow::ensure!(
            is_unique_set(config.starboard.values().flat_map(|b| b.boards.keys())),
            "starboard ids must be globally unique",
        );

        anyhow::ensure!(
            config.mongodb_uri.is_some(),
            "starboard requires a mongodb_uri",
        );

        log::info!("Starboard is enabled: {} guild(s)", config.starboard.len());

        Ok(())
    }

    async fn db_init(self, _data: Arc<HBotData>, db: mongodb::Database) -> Result {
        use model::*;

        use crate::helper::bson::update_indices;
        update_indices(Message::collection(&db), Message::indices()).await?;
        update_indices(Score::collection(&db), Score::indices()).await?;
        Ok(())
    }
}

fn get_board(
    config: &HBotConfig,
    guild: GuildId,
    board: BoardId,
) -> Result<&config::StarboardEntry> {
    config
        .starboard
        .get(&guild)
        .context("starboard not configured for this guild")?
        .boards
        .get(&board)
        .context("starboard not found")
}

pub async fn reaction_add(ctx: Context, reaction: Reaction) {
    let message_link =
        MessageLink::new(reaction.guild_id, reaction.channel_id, reaction.message_id);

    if let Err(why) = reaction_add_inner(ctx, reaction).await {
        log::error!("Reaction handling failed for {message_link:#}: {why:?}");
    }
}

pub async fn message_delete(
    ctx: Context,
    channel_id: ChannelId,
    message_id: MessageId,
    guild_id: Option<GuildId>,
) {
    let Some(guild_id) = guild_id else {
        return;
    };

    let message_link = MessageLink::new(guild_id, channel_id, message_id);

    if let Err(why) = message_delete_inner(ctx, guild_id, channel_id, message_id).await {
        log::error!("Message delete handling failed for {message_link:#}: {why:?}");
    }
}

async fn reaction_add_inner(ctx: Context, reaction: Reaction) -> Result {
    // only in guilds
    // i'd also check for bots but... that's not in the reaction event
    let Some(guild_id) = reaction.guild_id else {
        return Ok(());
    };

    // look up the board associated with the emoji
    // note: the emoji name is part of the reaction data
    let data = ctx.data_ref::<HContextData>();

    // grab the config for the current guild
    let guild_config = data.config().starboard.get(&guild_id);
    let Some(guild_config) = guild_config else {
        return Ok(());
    };

    // ignore messages in board channels
    let is_board = guild_config
        .boards
        .values()
        .any(|b| b.channel == reaction.channel_id);

    if is_board {
        return Ok(());
    }

    let board = guild_config
        .boards
        .iter()
        .find(|b| b.1.emoji.equivalent_to(&reaction.emoji));

    let Some((&board_id, board)) = board else {
        return Ok(());
    };

    // avoid using the cache here even if it is enabled
    // we want to ensure that we have the fresh current state
    let message = reaction
        .message(&ctx.http)
        .await
        .context("could not load full message")?;

    // cannot starboard yourself
    // there are checks further down to ignore the user's reaction later on
    if message.author.id == reaction.user_id.context("user always set in react")? {
        return Ok(());
    }

    let reaction = message
        .reactions
        .iter()
        .find(|r| board.emoji.equivalent_to(&r.reaction_type))
        .context("could not find message reaction data")?;

    let db = data.database()?;
    let mut new_post = false;
    let score_increase;
    {
        // update the message document, if we have enough reacts
        let required_reacts = i64::from(board.reacts);

        // get the current reaction count
        // discount the bot's own reactions including supers,
        // even though bots can't add them anymore
        let mut now_reacts = i64::try_from(reaction.count)?;
        if reaction.me || reaction.me_burst {
            now_reacts -= 1;
        }

        // check the reacts once here already so we don't ask discord for the reacting
        // user even when there is no chance that it matters
        if now_reacts < required_reacts {
            return Ok(());
        }

        // if the author of this message has reacted, we subtract 1 from the count
        // so their own reaction does not contribute score
        // if there are super reactions, also check there
        let has_self_reaction = |burst| {
            has_reaction_by_user(
                &ctx,
                &message,
                &reaction.reaction_type,
                message.author.id,
                burst,
            )
        };
        let has_self_reaction = has_self_reaction(false).await?
            || (reaction.count_details.burst != 0 && has_self_reaction(true).await?);

        if has_self_reaction {
            now_reacts -= 1;
        }

        // this block is important because it otherwise pins every message
        if now_reacts < required_reacts {
            return Ok(());
        }

        let message_link = MessageLink::from(&message).guild_id(guild_id);
        log::debug!(
            "Trying update {message_link:#} in `{}` with {} total reacts.",
            board.name,
            now_reacts
        );

        let filter = model::Message::filter()
            .board(board_id)
            .message(message.id)
            .into_document()?;

        let update = model::Message::update()
            .set_on_insert(|m| {
                m.board(board_id)
                    .channel(message.channel_id)
                    .message(message.id)
                    .user(message.author.id)
                    .pinned(false)
            })
            .max(|m| m.max_reacts(now_reacts))
            .into_document()?;

        let record = model::Message::collection(db)
            .find_one_and_update(filter.clone(), update)
            .upsert(true)
            .return_document(ReturnDocument::Before)
            .await
            .context("failed to update message max_reacts")?;

        let (pinned, old_reacts) = record.map(|r| (r.pinned, r.max_reacts)).unwrap_or_default();

        // the score is the new amount compared to the old one
        // if it's now less, we return it as zero
        score_increase = now_reacts.saturating_sub(old_reacts).max(0);

        log::debug!(
            "Score for {message_link:#} in `{}` increased by {}.",
            board.name,
            score_increase
        );

        // we already checked that we have the required reacts,
        // so as long it wasn't already pinned, we can do that now
        if !pinned {
            // update the record to be pinned
            let update = model::Message::update()
                .set(|w| w.pinned(true))
                .into_document()?;

            let record = model::Message::collection(db)
                .find_one_and_update(filter.clone(), update)
                .return_document(ReturnDocument::Before)
                .await
                .context("failed to update message pin state")?
                .context("expected to find record that was just created")?;

            // pin the message if the update just now changed the value
            if !record.pinned {
                new_post = true;
                pin_message_to_board(&ctx, &message, guild_id, guild_config, board, filter).await?;
            }
        }
    };

    if score_increase > 0 {
        // update the user's score if it has increased
        let filter = model::Score::filter()
            .board(board_id)
            .user(message.author.id)
            .into_document()?;

        let update = model::Score::update()
            .set_on_insert(|s| s.board(board_id).user(message.author.id))
            .inc(|s| s.score(score_increase).post_count(i64::from(new_post)))
            .into_document()?;

        model::Score::collection(db)
            .update_one(filter, update)
            .upsert(true)
            .await
            .context("failed to update user score")?;

        log::trace!(
            "{} gained {} `{}`.",
            message.author.name,
            score_increase,
            board.name
        );

        if board.any_cash_gain() && super::perks::Module.enabled(data.config()) {
            use super::perks::Item;
            use super::perks::model::{Wallet, WalletExt};

            let amount = score_increase
                .saturating_mul(board.cash_gain.into())
                .saturating_add(if new_post {
                    board.cash_pin_gain.into()
                } else {
                    0
                });

            Wallet::collection(db)
                .add_items(guild_id, message.author.id, Item::Cash, amount)
                .await?;

            log::trace!("{} gained {} cash.", message.author.name, amount);
        }
    }

    Ok(())
}

async fn message_delete_inner(
    ctx: Context,
    guild_id: GuildId,
    channel_id: ChannelId,
    message_id: MessageId,
) -> Result {
    let data = ctx.data_ref::<HContextData>();

    // grab the config for the current guild
    let guild_config = data.config().starboard.get(&guild_id);

    let Some(guild_config) = guild_config else {
        return Ok(());
    };

    // skip if we don't remove score in this guild
    if !guild_config.remove_score_on_delete {
        return Ok(());
    }

    let db = data.database()?;

    // look for all boards with the message and iterate the entries
    let filter = model::Message::filter()
        .board(Filter::in_(guild_config.boards.keys().copied()))
        .message(message_id)
        .into_document()?;

    let mut query = model::Message::collection(db)
        .find(filter)
        .await
        .context("failed to begin message query")?;

    while let Some(item) = query.next().await {
        let item = item.context("failed to get next deletion entry")?;

        // we need the board info, skip if we don't know it
        let board = guild_config.boards.get(&item.board);
        let Some(board) = board else {
            continue;
        };

        let message_link = MessageLink::new(guild_id, channel_id, message_id);
        log::debug!(
            "Trying delete of {message_link:#} by {} in `{}` with {} reacts.",
            item.user,
            board.name,
            item.max_reacts
        );

        let filter = model::Score::filter()
            .board(item.board)
            .user(item.user)
            .into_document()?;

        let update = model::Score::update()
            .inc(|s| s.score(-item.max_reacts).post_count(-1))
            .into_document()?;

        // delete the message tracking entry
        model::Message::collection(db)
            .delete_one(item.self_filter())
            .await
            .context("failed to delete message entry")?;

        log::info!(
            "Deleted message {message_link:#} score in `{}`.",
            board.name
        );

        // delete the associated pins
        for pin_id in item.pin_messages {
            let res = board
                .channel
                .delete_message(&ctx.http, pin_id, Some("pin source deleted"))
                .await;

            if let Err(why) = res {
                log::warn!(
                    "Failed to delete message {:#} in `{}`: {why:?}",
                    MessageLink::new(guild_id, board.channel, pin_id),
                    board.name
                );
            }
        }

        // update the user score
        model::Score::collection(db)
            .update_one(filter, update)
            .await
            .context("failed to reduce user score")?;

        log::trace!("{} lost {} `{}`.", item.user, item.max_reacts, board.name);

        // also remove cash if it's configured
        if board.any_cash_gain() && super::perks::Module.enabled(data.config()) {
            use super::perks::Item;
            use super::perks::model::{Wallet, WalletExt};

            let amount = item
                .max_reacts
                .saturating_mul(board.cash_gain.into())
                .saturating_add(if item.pinned {
                    board.cash_pin_gain.into()
                } else {
                    0
                });

            Wallet::collection(db)
                .add_items(guild_id, item.user, Item::Cash, -amount)
                .await?;

            log::trace!("{} lost {} cash.", item.user, amount);
        }
    }

    Ok(())
}

async fn pin_message_to_board(
    ctx: &Context,
    message: &Message,
    guild_id: GuildId,
    guild_config: &config::StarboardGuild,
    board: &config::StarboardEntry,
    filter: bson::Document,
) -> Result {
    let data = ctx.data_ref::<HContextData>();
    let db = data.database()?;

    // unless it's nsfw-to-sfw, actually forward the message
    // otherwise, generate an embed with a link
    let can_forward = is_forwarding_allowed(ctx, message, board)
        .await
        .unwrap_or(false);

    // guard sending the pin messages per-guild so they don't interleave. this
    // shouldn't lead to much contention (and even then this code isn't hot).
    // ideally, this would be per-channel, but there isn't a good, easy place to
    // store that. the await isn't further down so the `CreateMessage` doesn't need
    // to be lifted into the Future and allocations live for less time.
    let pin_guard = guild_config
        .pin_lock
        .acquire()
        .await
        .expect("pin_lock should never be closed");

    let notice = board
        .notices
        .choose(&mut rand::rng())
        .map(String::as_str)
        .unwrap_or("{user}, your post made it! Wow!");

    let notice = replace_holes(notice, |out, n| match n {
        "user" => write_str!(out, "<@{}>", message.author.id),
        _ => out.push(char::REPLACEMENT_CHARACTER),
    });

    let notice = CreateMessage::new().content(notice);
    let message_link = MessageLink::from(message).guild_id(guild_id);

    let pin_messages;
    if can_forward {
        let notice = board
            .channel
            .send_message(&ctx.http, notice)
            .await
            .context("could not send pin notice")?
            .id;

        let mut forward = MessageReference::from(message);
        forward.kind = MessageReferenceKind::Forward;

        let forward = CreateMessage::new().reference_message(forward);
        let forward = board
            .channel
            .send_message(&ctx.http, forward)
            .await
            .context("could not send pin forward")?
            .id;

        pin_messages = vec![notice, forward];
        log::info!(
            "Pinned message {message_link:#} to `{}`. (Forward)",
            board.name
        );
    } else {
        // nsfw-to-sfw
        let forward = format!("🔞 {message_link}");
        let forward = CreateEmbed::new()
            .description(forward)
            .color(data.config().embed_color)
            .timestamp(message.timestamp);

        let notice = notice.embed(forward);
        let notice = board
            .channel
            .send_message(&ctx.http, notice)
            .await
            .context("could not send pin notice+link")?
            .id;

        pin_messages = vec![notice];
        log::info!(
            "Pinned message {message_link:#} to `{}`. (Link)",
            board.name
        );
    }

    // release the semaphore here so the db operation won't delay it
    drop(pin_guard);

    // also associate what messages are the pins
    let update = model::Message::update()
        .set(|m| m.pin_messages(pin_messages))
        .into_document()?;

    model::Message::collection(db)
        .update_one(filter, update)
        .await
        .context("failed to set message pin_messages")?;
    Ok(())
}

async fn has_reaction_by_user(
    ctx: &Context,
    message: &Message,
    emoji: &ReactionType,
    user_id: UserId,
    burst: bool,
) -> Result<bool> {
    use arrayvec::ArrayVec;
    use serenity::http::{LightMethod, Request, Route};
    use to_arraystring::ToArrayString;

    let after = UserId::new(user_id.get().saturating_sub(1));
    let after_str = after.to_arraystring();

    // we grab a single user after the reacting user's id
    // to check whether they added this kind of reaction
    let params = [
        ("limit", "1"),
        ("after", &after_str),
        ("type", if burst { "1" } else { "0" }),
    ];

    let route = Route::ChannelMessageReactionEmoji {
        channel_id: message.channel_id,
        message_id: message.id,
        reaction: &emoji.as_data(),
    };

    // since we only grab 1 user at most, use `ArrayVec` to avoid an allocation
    let request = Request::new(route, LightMethod::Get).params(&params);
    let reacted_users: ArrayVec<User, 1> = ctx
        .http
        .fire(request)
        .await
        .context("could not get user self-react")?;

    Ok(reacted_users.first().is_some_and(|u| u.id == user_id))
}

async fn is_forwarding_allowed(
    ctx: &Context,
    message: &Message,
    board: &config::StarboardEntry,
) -> serenity::Result<bool> {
    let source = message
        .channel_id
        .to_guild_channel(ctx, message.guild_id)
        .await?;

    if !source.nsfw {
        return Ok(true);
    }

    let target = board
        .channel
        .to_guild_channel(ctx, message.guild_id)
        .await?;

    // at this point, the source channel is nsfw,
    // so to allow forwarding, the target must also be nsfw
    Ok(target.nsfw)
}
