use anyhow::Context as _;
use bson::doc;
use mongodb::options::ReturnDocument;
use rand::prelude::*;
use serenity::futures::TryStreamExt;
use serenity::prelude::*;

use super::Module as _;
use crate::helper::bson::{bson_id, doc_object_id};
use crate::helper::is_unique_set;
use crate::prelude::*;
use crate::config::HBotConfig;

pub mod buttons;
pub mod config;
pub mod model;
mod slashies;

pub use config::{Config, BoardId};

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &HBotConfig) -> bool {
        !config.starboard.is_empty()
    }

    fn intents(&self, _config: &HBotConfig) -> GatewayIntents {
        GatewayIntents::GUILD_MESSAGE_REACTIONS |
        GatewayIntents::GUILD_MESSAGES
    }

    fn commands(&self, _config: &HBotConfig) -> impl IntoIterator<Item = HCommand> {
        [
            slashies::starboard()
        ]
    }

    fn db_init(db: &mongodb::Database) -> mongodb::BoxFuture<'_, HResult> {
        Box::pin(async move {
            model::Message::collection(db).create_indexes(model::Message::indices()).await?;
            model::Score::collection(db).create_indexes(model::Score::indices()).await?;
            Ok(())
        })
    }

    fn validate(&self, config: &HBotConfig) -> HResult {
        anyhow::ensure!(
            is_unique_set(config.starboard.values().flat_map(|b| b.boards.keys())),
            "starboard ids must be globally unique",
        );

        if config.mongodb_uri.is_none() {
            anyhow::bail!("starboard requires a mongodb_uri");
        }

        log::info!("Starboard is enabled: {} guild(s)", config.starboard.len());

        Ok(())
    }
}

fn get_board(config: &HBotConfig, guild: GuildId, board: BoardId) -> anyhow::Result<&config::StarboardEntry> {
    config.starboard
        .get(&guild)
        .context("starboard not configured for this guild")?
        .boards
        .get(&board)
        .context("starboard not found")
}

pub async fn reaction_add(ctx: Context, reaction: Reaction) {
    if let Err(why) = reaction_add_inner(ctx, reaction).await {
        log::error!("Reaction handling failed: {why:?}");
    }
}

pub async fn message_delete(ctx: Context, channel_id: ChannelId, message_id: MessageId, guild_id: Option<GuildId>) {
    let Some(guild_id) = guild_id else {
        return;
    };

    if let Err(why) = message_delete_inner(ctx, guild_id, channel_id, message_id).await {
        log::error!("Message delete handling failed: {why:?}");
    }
}

async fn reaction_add_inner(ctx: Context, reaction: Reaction) -> HResult {
    // only in guilds
    let Some(guild_id) = reaction.guild_id else {
        return Ok(());
    };

    // look up the board associated with the emoji
    // note: the emoji name is part of the reaction data
    let data = ctx.data_ref::<HFrameworkData>();

    // grab the config for the current guild
    let guild_config = data.config()
        .starboard
        .get(&guild_id);

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

    let Some((board_id, board)) = board else {
        return Ok(());
    };

    // avoid using the cache here even if it is enabled
    // we want to ensure that we have the fresh current state
    let message = reaction.message(&ctx.http).await?;

    // cannot starboard yourself
    // there are checks further down to ignore the user's reaction later on
    if message.author.id == reaction.user_id.context("user always set in react")? {
        return Ok(());
    }

    let reaction = message.reactions
        .iter()
        .find(|r| r.reaction_type == reaction.emoji)
        .context("could not find reaction data")?;

    let db = data.database()?;
    let mut new_post = false;
    let score_increase = {
        // update the message document, if we have enough reacts
        let required_reacts = i64::from(board.reacts);
        let mut now_reacts = i64::try_from(reaction.count)?;
        if now_reacts < required_reacts {
            return Ok(());
        }

        // we grab a single user after the reacting user's id
        // if this is the reacting user, we subtract 1 from the count
        // so their own reaction does not contribute score
        let reacted_users = message.reaction_users(
            &ctx.http,
            reaction.reaction_type.clone(),
            Some(1), // limit: we just need the next one
            Some(UserId::new(message.author.id.get().saturating_sub(1))),
        ).await?;

        if reacted_users.iter().any(|u| u.id == message.author.id) {
            now_reacts -= 1;
        }

        // we may now have less reacts than needed
        if now_reacts < required_reacts {
            return Ok(());
        }

        let filter = doc! {
            "board": board_id.get(),
            "message": bson_id!(message.id),
        };

        let update = doc! {
            "$setOnInsert": {
                "board": board_id.get(),
                "channel": bson_id!(message.channel_id),
                "message": bson_id!(message.id),
                "user": bson_id!(message.author.id),
                "pinned": false,
            },
            "$max": {
                "max_reacts": now_reacts,
            },
        };

        let record = model::Message::collection(db)
            .find_one_and_update(filter.clone(), update)
            .upsert(true)
            .return_document(ReturnDocument::Before)
            .await?;

        let (pinned, old_reacts) = record
            .map(|r| (r.pinned, r.max_reacts))
            .unwrap_or_default();

        // we already checked that we have the required reacts, but for sanity, keep it here
        if now_reacts >= required_reacts && !pinned {
            // update the record to be pinned
            let update = doc! {
                "$set": {
                    "pinned": true,
                },
            };

            let record = model::Message::collection(db)
                .find_one_and_update(filter.clone(), update)
                .return_document(ReturnDocument::Before)
                .await?
                .context("expected to find record that was just created")?;

            // pin the message if the update just now changed the value
            if !record.pinned {
                new_post = true;

                let notice = board.notices
                    .choose(&mut thread_rng())
                    .map(String::as_str)
                    .unwrap_or("{user}, your post made it! Wow!");

                let notice = CreateMessage::new()
                    .content(notice.replace("{user}", &format!("<@{}>", message.author.id)));

                let pin_messages;

                // unless it's nsfw-to-sfw, actually forward the message
                // otherwise, generate an embed with a link
                if is_forwarding_allowed(&ctx, &message, board).await.unwrap_or(false) {
                    // CMBK: refactor when forwards are properly supported
                    let mut forward = MessageReference::from(&message);
                    forward.kind = MessageReferenceKind::Forward;

                    let forward = CreateMessage::new()
                        .reference_message(forward);

                    let notice = board.channel.send_message(&ctx.http, notice).await?.id;
                    let forward = board.channel.send_message(&ctx.http, forward).await?.id;
                    pin_messages = vec![bson_id!(notice), bson_id!(forward)];
                    log::info!("Pinned message {} to {}.", message.id, board.emoji.name());
                } else {
                    // nsfw-to-sfw
                    let forward = format!(
                        "🔞 https://discord.com/channels/{}/{}/{}",
                        guild_id, message.channel_id, message.id,
                    );

                    let forward = CreateEmbed::new()
                        .description(forward)
                        .color(data.config().embed_color)
                        .timestamp(message.timestamp);

                    let notice = notice
                        .embed(forward);

                    let notice = board.channel.send_message(&ctx.http, notice).await?.id;
                    pin_messages = vec![bson_id!(notice)];
                    log::info!("Pinned message {} to {}. (Link)", message.id, board.emoji.name());
                }

                // also associate what messages are the pins
                let update = doc! {
                    "$set": {
                        "pin_messages": pin_messages,
                    },
                };

                model::Message::collection(db)
                    .update_one(filter, update)
                    .await?;
            }
        }

        // the score is the new amount compared to the old one
        // if it's now less, we return it as zero
        now_reacts.saturating_sub(old_reacts)
    };

    if score_increase > 0 {
        // update the user's score if it has increased
        let filter = doc! {
            "board": board_id.get(),
            "user": bson_id!(message.author.id),
        };

        let update = doc! {
            "$setOnInsert": {
                "board": board_id.get(),
                "user": bson_id!(message.author.id),
            },
            "$inc": {
                "score": score_increase,
                "post_count": i64::from(new_post),
            },
        };

        model::Score::collection(db)
            .update_one(filter, update)
            .upsert(true)
            .await?;

        log::trace!("{} gained {} {}.", message.author.name, score_increase, board.emoji.name());

        if board.cash_gain != 0 && super::perks::Module.enabled(data.config()) {
            use super::perks::model::{Wallet, WalletExt};
            use super::perks::Item;

            let amount = i64::from(board.cash_gain).saturating_mul(score_increase);

            Wallet::collection(db)
                .add_items(guild_id, message.author.id, Item::Cash, amount)
                .await?;

            log::trace!("{} gained {} cash.", message.author.name, amount);
        }
    }

    Ok(())
}

async fn message_delete_inner(ctx: Context, guild_id: GuildId, _channel_id: ChannelId, message_id: MessageId) -> HResult {
    let data = ctx.data_ref::<HFrameworkData>();

    // grab the config for the current guild
    let guild_config = data.config()
        .starboard
        .get(&guild_id);

    let Some(guild_config) = guild_config else {
        return Ok(());
    };

    // skip if we don't remove score in this guild
    if !guild_config.remove_score_on_delete {
        return Ok(());
    }

    let db = data.database()?;

    // look for all boards with the message and iterate the entries
    let board_ids: Vec<_> = guild_config
        .boards
        .keys()
        .map(|b| b.get())
        .collect();

    let filter = doc! {
        "board": {
            "$in": board_ids,
        },
        "message": bson_id!(message_id),
    };

    let mut query = model::Message::collection(db)
        .find(filter)
        .await?;

    while let Some(item) = query.try_next().await? {
        // we need the board info, skip if we don't know it
        let board = guild_config
            .boards
            .get(&item.board);

        let Some(board) = board else {
            continue;
        };

        let filter = doc! {
            "board": item.board.get(),
            "user": bson_id!(item.user),
        };

        let update = doc! {
            "$inc": {
                "score": -item.max_reacts,
                "post_count": -1,
            },
        };

        // delete the message tracking entry
        model::Message::collection(db)
            .delete_one(doc_object_id!(item))
            .await?;

        log::info!("Deleted message {} score in {}.", message_id, board.emoji);

        // update the user score
        model::Score::collection(db)
            .update_one(filter, update)
            .await?;

        log::trace!("{} lost {} {}.", item.user, item.max_reacts, board.emoji);

        // delete the associated pins
        for pin_id in item.pin_messages {
            if let Err(why) = board.channel.delete_message(&ctx.http, pin_id, Some("pin source deleted")).await {
                log::warn!("Failed to delete message {pin_id} in {}: {why:?}", board.emoji);
            }
        }

        // also remove cash if it's configured
        if board.cash_gain != 0 && super::perks::Module.enabled(data.config()) {
            use super::perks::model::{Wallet, WalletExt};
            use super::perks::Item;

            let amount = i64::from(board.cash_gain).saturating_mul(item.max_reacts);

            Wallet::collection(db)
                .add_items(guild_id, item.user, Item::Cash, -amount)
                .await?;

            log::trace!("{} lost {} cash.", item.user, amount);
        }
    }

    Ok(())
}

async fn is_forwarding_allowed(ctx: &Context, message: &Message, board: &config::StarboardEntry) -> anyhow::Result<bool> {
    let source = message
        .channel_id
        .to_guild_channel(ctx, message.guild_id).await?;

    if !source.nsfw {
        return Ok(true);
    }

    let target = board
        .channel
        .to_guild_channel(ctx, message.guild_id).await?;

    // at this point, the source channel is nsfw,
    // so to allow forwarding, the target must also be nsfw
    Ok(target.nsfw)
}
