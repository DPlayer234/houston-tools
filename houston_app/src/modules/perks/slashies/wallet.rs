use utils::text::write_str::*;

use crate::modules::perks::Item;
use crate::modules::perks::model::Wallet;
use crate::slashies::prelude::*;

/// View your server wallet.
#[chat_command(contexts = "Guild", integration_types = "Guild")]
pub async fn wallet(
    ctx: Context<'_>,
    /// Whether to show the response only to yourself.
    ephemeral: Option<bool>,
) -> Result {
    let data = ctx.data_ref();
    let guild_id = ctx.require_guild_id()?;
    let perks = data.config().perks()?;
    let db = data.database()?;

    ctx.defer_as(ephemeral).await?;

    let filter = Wallet::filter()
        .guild(guild_id)
        .user(ctx.user().id)
        .into_document()?;

    let wallet = Wallet::collection(db)
        .find_one(filter)
        .await?
        .unwrap_or_default();

    let mut description = String::new();

    for &item in Item::all() {
        let owned = wallet.item(item);
        if owned != 0 {
            let name = item.info(perks).name;
            writeln_str!(description, "- **{name}:** x{owned}");
        }
    }

    let description = crate::fmt::written_or(description, "<None>");

    let (display_name, face) = get_display_info(ctx);
    let author = format!("{display_name}: Wallet");
    let author = CreateEmbedAuthor::new(author).icon_url(face);

    let embed = CreateEmbed::new()
        .author(author)
        .color(data.config().embed_color)
        .description(description);

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}

fn get_display_info(ctx: Context<'_>) -> (&str, String) {
    match ctx.member() {
        Some(member) => (member.display_name(), member.face()),
        _ => (ctx.user().display_name(), ctx.user().face()),
    }
}
