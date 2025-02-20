use bson::doc;
use bson_model::Filter;
use bson_model::Sort::Desc;
use utils::text::write_str::*;

use crate::fmt::discord::MessageLink;
use crate::helper::bson::id_as_i64;
use crate::modules::starboard::{model, BoardId};
use crate::slashies::prelude::*;

#[derive(Debug, serde::Deserialize)]
struct TopScore {
    #[serde(rename = "_id")]
    board: BoardId,
    #[serde(with = "id_as_i64")]
    user: UserId,
    #[serde(default)]
    score: i64,
    #[serde(default)]
    post_count: i64,
}

#[derive(Debug, serde::Deserialize)]
struct TopMessage {
    #[serde(rename = "_id")]
    board: BoardId,
    #[serde(with = "id_as_i64")]
    channel: ChannelId,
    #[serde(with = "id_as_i64")]
    message: MessageId,
    #[serde(with = "id_as_i64")]
    user: UserId,
    #[serde(default)]
    max_reacts: i64,
}

pub async fn overview(ctx: Context<'_>, ephemeral: Option<bool>) -> Result {
    let guild = ctx.require_guild_id()?;
    let data = ctx.data_ref();
    let db = data.database()?;
    let guild_config = data
        .config()
        .starboard
        .get(&guild)
        .ok_or(HArgError::new_const(
            "Starboard is not enabled for this server.",
        ))?;

    ctx.defer_as(ephemeral).await?;

    let top_posts = model::Message::collection(db)
        .aggregate([
            doc! {
                "$match": model::Message::filter()
                    .board(Filter::in_(guild_config.boards.keys().copied()))
                    .into_document()?,
            },
            doc! {
                "$sort": model::Message::sort()
                    .max_reacts(Desc)
                    .message(Desc)
                    .into_document(),
            },
            doc! {
                "$group": {
                    "_id": "$board",
                    "channel": { "$first": "$channel" },
                    "message": { "$first": "$message" },
                    "user": { "$first": "$user" },
                    "max_reacts": { "$max": "$max_reacts" },
                },
            },
        ])
        .with_type::<TopMessage>()
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let top_users = model::Score::collection(db)
        .aggregate([
            doc! {
                "$match": model::Score::filter()
                    .board(Filter::in_(guild_config.boards.keys().copied()))
                    .into_document()?,
            },
            doc! {
                "$sort": model::Score::sort()
                    .score(Desc)
                    .post_count(Desc)
                    .into_document(),
            },
            doc! {
                "$group": {
                    "_id": "$board",
                    "user": { "$first": "$user" },
                    "score": { "$max": "$score" },
                    "post_count": { "$first": "$post_count" },
                },
            },
        ])
        .with_type::<TopScore>()
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let mut embed = CreateEmbed::new()
        .title("Starboard Overview")
        .color(data.config().embed_color);

    for (id, board) in &guild_config.boards {
        let mut value = String::with_capacity(256);

        value.push_str("- **Top Post:** ");
        match top_posts.iter().find(|t| t.board == *id) {
            Some(top_post) => writeln_str!(
                value,
                "{} by <@{}>: {} {}",
                MessageLink::new(guild, top_post.channel, top_post.message),
                top_post.user,
                top_post.max_reacts,
                board.emoji,
            ),
            None => value.push_str("<None>\n"),
        }

        value.push_str("- **Top Poster:** ");
        match top_users.iter().find(|t| t.board == *id) {
            Some(top_user) => write_str!(
                value,
                "<@{}>: {} {} from {} post(s)",
                top_user.user,
                top_user.score,
                board.emoji,
                top_user.post_count,
            ),
            None => value.push_str("<None>"),
        }

        embed = embed.field(format!("{} {}", board.emoji, board.name), value, false);
    }

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}
