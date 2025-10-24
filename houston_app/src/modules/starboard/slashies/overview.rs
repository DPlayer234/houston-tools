use bson::doc;
use bson_model::Sort::Desc;
use bson_model::{Filter, ModelDocument};
use serde_with::As;
use utils::text::WriteStr as _;

use crate::helper::bson::IdBson;
use crate::modules::starboard::{BoardId, Resources, model};
use crate::slashies::prelude::*;

#[derive(Debug, serde::Deserialize, ModelDocument)]
#[model(fields_only)]
struct TopScore {
    #[serde(rename = "_id")]
    board: BoardId,
    #[serde(with = "As::<IdBson>")]
    user: UserId,
    #[serde(default)]
    score: i64,
    #[serde(default)]
    post_count: i64,
}

#[derive(Debug, serde::Deserialize, ModelDocument)]
#[model(fields_only)]
struct TopMessage {
    #[serde(rename = "_id")]
    board: BoardId,
    #[serde(with = "As::<IdBson>")]
    channel: GenericChannelId,
    #[serde(with = "As::<IdBson>")]
    message: MessageId,
    #[serde(with = "As::<IdBson>")]
    user: UserId,
    #[serde(default)]
    max_reacts: i64,
}

/// Shows an overview of all boards.
#[sub_command]
pub async fn overview(
    ctx: Context<'_>,
    /// Whether to show the response only to yourself.
    ephemeral: Option<bool>,
) -> Result {
    let guild = ctx.require_guild_id()?;
    let data = ctx.data_ref();
    let db = data.database()?;
    let res = Resources::request_locale();

    let guild_config = data
        .config()
        .starboard
        .get(&guild)
        .ok_or_else(|| HArgError::new(res.error_not_enabled().build()))?;

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
                    // `TopMessage::board` is the group key/_id field
                    TopMessage::fields().board(): model::Message::fields().board(),
                    TopMessage::fields().channel(): { "$first": model::Message::fields().channel() },
                    TopMessage::fields().message(): { "$first": model::Message::fields().message() },
                    TopMessage::fields().user(): { "$first": model::Message::fields().user() },
                    TopMessage::fields().max_reacts(): { "$max": model::Message::fields().max_reacts() },
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
                    // board is the group key/_id field
                    TopScore::fields().board(): model::Score::fields().board(),
                    TopScore::fields().user(): { "$first": model::Score::fields().user() },
                    TopScore::fields().score(): { "$max": model::Score::fields().score() },
                    TopScore::fields().post_count(): { "$first": model::Score::fields().post_count() },
                },
            },
        ])
        .with_type::<TopScore>()
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let mut embed = CreateEmbed::new()
        .title(res.overview().header().build())
        .color(data.config().embed_color);

    for board in guild_config.boards.values() {
        let mut value = String::with_capacity(256);

        write!(value, "- **{}:** ", res.overview().top_post().build());
        match top_posts.iter().find(|t| t.board == board.id) {
            Some(top_post) => writeln!(
                value,
                "{}",
                res.post_by_user()
                    .link(top_post.message.link(top_post.channel, Some(guild)))
                    .user(top_post.user.mention())
                    .max_reacts(top_post.max_reacts)
                    .emoji(board.emoji())
                    .build(),
            ),
            None => writeln!(value, "{}", res.no_content().build()),
        }

        write!(value, "- **{}:** ", res.overview().top_poster().build());
        match top_users.iter().find(|t| t.board == board.id) {
            Some(top_user) => write!(
                value,
                "{}",
                res.user_score()
                    .user(top_user.user.mention())
                    .score(top_user.score)
                    .emoji(board.emoji())
                    .post_count(top_user.post_count)
                    .build()
            ),
            None => write!(value, "{}", res.no_content().build()),
        }

        embed = embed.field(format!("{} {}", board.emoji(), board.name), value, false);
    }

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}
