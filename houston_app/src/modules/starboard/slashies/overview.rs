use bson::doc;

use utils::text::write_str::*;

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

    let filter = doc! {
        "board": {
            "$in": guild_config.board_db_keys(),
        },
    };

    let sort = doc! {
        "max_reacts": -1,
    };

    let top_posts = model::Message::collection(db)
        .find(filter.clone())
        .sort(sort)
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let sort = doc! {
        "score": -1,
    };

    let top_users = model::Score::collection(db)
        .find(filter)
        .sort(sort)
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let mut embed = CreateEmbed::new()
        .title("Starboard Overview")
        .color(data.config().embed_color);

    for (id, board) in &guild_config.boards {
        let mut value = String::with_capacity(256);

        write_str!(value, "- **Top Post:** ");
        match top_posts.iter().find(|m| m.board == *id) {
            Some(top_post) => writeln_str!(
                value,
                "https://discord.com/channels/{}/{}/{} by <@{}>: {} {}",
                guild, top_post.channel, top_post.message, top_post.user, top_post.max_reacts, board.emoji,
            ),
            None => writeln_str!(value, "<None>"),
        }

        write_str!(value, "- **Top Poster:** ");
        match top_users.iter().find(|m| m.board == *id) {
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
