use anyhow::Context as _;
use bson::doc;
use mongodb::options::ReturnDocument;
use rand::prelude::*;
use serenity::prelude::*;

use crate::helper::bson_id;
use crate::modules::Module as _;
use crate::prelude::*;

pub mod buttons;
pub mod model;
mod slashies;

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &config::HBotConfig) -> bool {
        !config.starboard.is_empty()
    }

    fn intents(&self) -> GatewayIntents {
        GatewayIntents::GUILD_MESSAGE_REACTIONS
    }

    fn commands(&self) -> impl IntoIterator<Item = HCommand> {
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

    fn validate(&self, config: &config::HBotConfig) -> HResult {
        if config.mongodb_uri.is_none() {
            anyhow::bail!("starboard requires a mongodb_uri");
        }

        log::info!("Starboard is enabled: {} board(s)", config.starboard.len());
        Ok(())
    }
}

pub type Config = Vec<StarboardEntry>;

#[derive(Debug, serde::Deserialize)]
pub struct StarboardEntry {
    pub name: String,
    pub guild: GuildId,
    pub channel: ChannelId,
    pub emoji: StarboardEmoji,
    pub reacts: u8,
    #[serde(default = "Vec::new")]
    pub notices: Vec<String>,
    #[serde(default)]
    pub cash_gain: i8,
}

#[derive(Debug)]
pub struct StarboardEmoji(ReactionType);

impl StarboardEmoji {
    pub fn as_emoji(&self) -> &ReactionType {
        &self.0
    }

    pub fn name(&self) -> &str {
        match self.as_emoji() {
            ReactionType::Custom { name, .. } => name.as_ref().expect("always set").as_str(),
            ReactionType::Unicode(unicode) => unicode.as_str(),
            _ => panic!("never set to invalid"),
        }
    }

    pub fn equivalent_to(&self, reaction: &ReactionType) -> bool {
        match (self.as_emoji(), reaction) {
            (ReactionType::Custom { id: self_id, .. }, ReactionType::Custom { id: other_id, .. }) => self_id == other_id,
            (ReactionType::Unicode(self_name), ReactionType::Unicode(other_name)) => self_name == other_name,
            _ => false,
        }
    }
}

impl<'de> serde::Deserialize<'de> for StarboardEmoji {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::fmt;

        use serenity::small_fixed_array::FixedString;

        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = StarboardEmoji;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("expected string for emoji")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let emoji = if let Some((id, name)) = v.split_once(':') {
                    let id = id.parse::<EmojiId>().map_err(|_| E::custom("invalid emoji id"))?;
                    ReactionType::Custom { animated: false, id, name: Some(FixedString::from_str_trunc(name)) }
                } else {
                    ReactionType::Unicode(FixedString::from_str_trunc(v))
                };

                Ok(StarboardEmoji(emoji))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

pub async fn handle_reaction(ctx: Context, reaction: Reaction) {
    if let Err(why) = handle_core(ctx, reaction).await {
        log::error!("Reaction handling failed: {why:?}");
    }
}

async fn handle_core(ctx: Context, reaction: Reaction) -> HResult {
    // look up the board associated with the emoji
    // note: the emoji name is part of the reaction data
    let data = ctx.data_ref::<HBotData>();

    // ignore messages in board channels
    let is_board = data.config()
        .starboard
        .iter()
        .any(|b| b.channel == reaction.channel_id);

    if is_board {
        return Ok(());
    }

    let board = data.config()
        .starboard
        .iter()
        .find(|b| b.emoji.equivalent_to(&reaction.emoji) && Some(b.guild) == reaction.guild_id);

    let Some(board) = board else {
        return Ok(());
    };

    let message = reaction.message(&ctx).await?;

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
            "board": bson_id!(board.channel),
            "message": bson_id!(message.id),
        };

        let update = doc! {
            "$setOnInsert": {
                "board": bson_id!(board.channel),
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
            .find_one_and_update(filter, update)
            .upsert(true)
            .return_document(ReturnDocument::Before)
            .await?;

        let (pinned, old_reacts) = record
            .map(|r| (r.pinned, r.max_reacts))
            .unwrap_or_default();

        // we already checked that we have the required reacts, but for sanity, keep it here
        if now_reacts >= required_reacts && !pinned {
            // update the record to pinned
            let filter = doc! {
                "board": bson_id!(board.channel),
                "message": bson_id!(message.id),
            };

            let update = doc! {
                "$set": {
                    "pinned": true,
                },
            };

            let record = model::Message::collection(db)
                .find_one_and_update(filter, update)
                .return_document(ReturnDocument::Before)
                .await?
                .context("expected to find record that was just created")?;

            // pin the message if the update just now changed the value
            // we also need to ignore nsfw channels
            if !record.pinned && !(is_in_nsfw(&ctx, &message).await?) {
                let notice = board.notices
                    .choose(&mut thread_rng())
                    .map(String::as_str)
                    .unwrap_or("{user}, your post made it! Wow!");

                let notice = CreateMessage::new()
                    .content(notice.replace("{user}", &format!("<@{}>", message.author.id)));

                // CMBK: replace with proper builder when forwarding is supported
                let forward = simd_json::json!({
                    "message_reference": {
                        "type": 1, // forward
                        "message_id": message.id,
                        "channel_id": message.channel_id,
                    }
                });

                board.channel.send_message(&ctx.http, notice).await?;
                ctx.http.send_message(board.channel, Vec::new(), &forward).await?;
                log::info!("Pinned message {} to {}.", message.id, board.emoji.name());
            }
        }

        // the score is the new amount compared to the old one
        // if it's now less, we return it as zero
        now_reacts.saturating_sub(old_reacts)
    };

    if score_increase > 0 {
        // update the user's score if it has increased
        let filter = doc! {
            "board": bson_id!(board.channel),
            "user": bson_id!(message.author.id),
        };

        let update = doc! {
            "$setOnInsert": {
                "board": bson_id!(board.channel),
                "user": bson_id!(message.author.id),
            },
            "$inc": {
                "score": score_increase,
            },
        };

        model::Score::collection(db)
            .update_one(filter, update)
            .upsert(true)
            .await?;

        log::trace!("{} gained {} {}.", message.author.name, score_increase, board.emoji.name());

        if board.cash_gain != 0 && super::perks::Module.enabled(data.config()) {
            use super::perks::model::{Wallet, WalletExt};

            let amount = i64::from(board.cash_gain).saturating_mul(score_increase);

            Wallet::collection(db)
                .add_cash(board.guild, message.author.id, amount)
                .await?;

            log::trace!("{} gained {} cash.", message.author.name, amount);
        }
    }

    Ok(())
}

async fn is_in_nsfw(ctx: &Context, message: &Message) -> anyhow::Result<bool> {
    let nsfw = message
        .channel_id
        .to_channel(ctx, message.guild_id).await?
        .guild()
        .is_some_and(|g| g.nsfw);

    Ok(nsfw)
}
