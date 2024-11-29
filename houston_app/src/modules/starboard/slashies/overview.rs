use bson::doc;

use utils::text::write_str::*;

use crate::helper::bson::bson_id;
use crate::modules::starboard::model;
use crate::slashies::prelude::*;

pub async fn overview(
    ctx: Context<'_>,
    ephemeral: Option<bool>,
) -> Result {
    let guild = ctx.require_guild_id()?;
    let data = ctx.data_ref();
    let db = data.database()?;
    let guild_config = data.config()
        .starboard
        .get(&guild)
        .ok_or(HArgError::new_const("Starboard is not enabled for this server."))?;

    ctx.defer_as(ephemeral).await?;

    let mut embed = CreateEmbed::new()
        .title("Starboard Overview")
        .color(data.config().embed_color);

    for (id, board) in &guild_config.boards {
        // CMBK: this might be possible to implement without 2 requests per board
        // maybe with an aggregate pipeline?
        let filter = doc! {
            "board": bson_id!(id),
        };

        let sort = doc! {
            "max_reacts": -1,
        };

        let top_post = model::Message::collection(db)
            .find_one(filter.clone())
            .sort(sort)
            .await?;

        let sort = doc! {
            "score": -1,
        };

        let top_user = model::Score::collection(db)
            .find_one(filter)
            .sort(sort)
            .await?;

        let mut value = String::with_capacity(256);

        write_str!(value, "- **Top Post:** ");
        match top_post {
            Some(top_post) => writeln_str!(
                value,
                "https://discord.com/channels/{}/{}/{} by <@{}>: {} {}",
                guild, top_post.channel, top_post.message, top_post.user, top_post.max_reacts, board.emoji,
            ),
            None => writeln_str!(value, "<None>"),
        }

        write_str!(value, "- **Top Poster:** ");
        match top_user {
            Some(top_user) => write_str!(
                value,
                "<@{}>: {} {}",
                top_user.user, top_user.score, board.emoji,
            ),
            None => write_str!(value, "<None>"),
        }

        embed = embed.field(
            format!("{} {}", board.emoji, board.name),
            value,
            false,
        );
    }

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}
