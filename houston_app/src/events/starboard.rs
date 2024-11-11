use serenity::prelude::*;

use crate::prelude::*;

pub const INTENTS: GatewayIntents = GatewayIntents::GUILD_MESSAGE_REACTIONS;

pub async fn handle_reaction(ctx: Context, reaction: Reaction) {
    #[cfg(feature = "db")]
    if let Err(why) = handle_core(ctx, reaction).await {
        log::error!("Reaction handling failed: {why:?}");
    }
}

#[cfg(feature = "db")]
pub async fn handle_core(ctx: Context, reaction: Reaction) -> HResult {
    use anyhow::Context;
    use mongodb::bson::doc;
    use mongodb::options::ReturnDocument;

    let reacted_emoji = match &reaction.emoji {
        ReactionType::Unicode(unicode) => unicode.as_str(),
        // only support unicode emojis for now
        _ => return Ok(()),
    };

    // look up the board associated with the emoji
    let data = ctx.data_ref::<HBotData>();
    let board = data.config()
        .starboard
        .iter()
        .find(|b| b.emoji == reacted_emoji);

    let Some(board) = board else {
        return Ok(());
    };

    // we can be in any channel except the board channel
    if board.channel == reaction.channel_id && Some(board.guild) == reaction.guild_id {
        return Ok(());
    }

    let message = reaction.message(&ctx).await?;

    // cannot starboard yourself
    if message.author.id == reaction.user_id.context("user always set in react")? {
        return Ok(());
    }

    let reaction = message.reactions
        .iter()
        .find(|r| r.reaction_type == reaction.emoji)
        .context("could not find reaction data")?;

    macro_rules! bson_id {
        ($expr:expr) => {{
            #[allow(clippy::cast_possible_wrap)]
            let value = $expr.get() as i64;
            value
        }};
    }

    let database = data.database()?;
    let score_increase = {
        // update the message document
        let mut now_reacts = i64::try_from(reaction.count)?;

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

        let filter = doc! {
            "board": bson_id!(board.channel),
            "message": bson_id!(message.id),
        };

        let update = doc! {
            "$setOnInsert": {
                "board": bson_id!(board.channel),
                "message": bson_id!(message.id),
                "user": bson_id!(message.author.id),
                "pinned": false,
            },
            "$max": {
                "max_reacts": now_reacts,
            },
        };

        let record = database.starboard_messages
            .find_one_and_update(filter, update)
            .upsert(true)
            .return_document(ReturnDocument::Before)
            .await?;

        let (pinned, old_reacts) = record
            .map(|r| (r.pinned, r.max_reacts))
            .unwrap_or_default();

        // check if we went above the required reactions and weren't already
        let required_reacts = i64::from(board.reacts);
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

            let record = database.starboard_messages
                .find_one_and_update(filter, update)
                .return_document(ReturnDocument::Before)
                .await?
                .context("expected to find record that was just created")?;

            // pin the message if the update just now changed the value
            if !record.pinned {
                // CMBK: replace with proper builder when forwarding is supported
                let payload = simd_json::json!({
                    "message_reference": {
                        "type": 1, // forward
                        "message_id": message.id,
                        "channel_id": message.channel_id,
                    }
                });

                ctx.http.send_message(board.channel, Vec::new(), &payload).await?;
                log::info!("Pinned message {} to {}.", message.id, board.emoji);
            }
        }

        // the score is the new amount compared to the old one
        // if it's now less, we return it as zero
        now_reacts.saturating_sub(old_reacts)
    };

    if score_increase > 0 {
        // update the user's score if it has increased
        let filter = doc! {
            "guild": bson_id!(board.guild),
            "board": bson_id!(board.channel),
            "user": bson_id!(message.author.id),
        };

        let update = doc! {
            "$setOnInsert": {
                "guild": bson_id!(board.guild),
                "board": bson_id!(board.channel),
                "user": bson_id!(message.author.id),
            },
            "$inc": {
                "score": score_increase,
            },
        };

        database.starboard_scores
            .update_one(filter, update)
            .upsert(true)
            .await?;

        log::trace!("{} gained {} {}.", message.author.name, score_increase, board.emoji);
    }

    Ok(())
}
