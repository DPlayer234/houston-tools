use utils::text::WriteStr as _;

use crate::fmt::StringExt as _;
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
    let member = ctx.member().context("must be used in guild (no member)")?;
    let perks = data.config().perks()?;
    let db = data.database()?;

    ctx.defer_as(ephemeral).await?;

    let filter = Wallet::filter()
        .guild(member.guild_id)
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
            writeln!(description, "- **{name}:** x{owned}");
        }
    }

    let description = description.or_default("<None>");

    let author = format!("{}: Wallet", member.display_name());
    let author = CreateEmbedAuthor::new(author).icon_url(member.face());

    let embed = CreateEmbed::new()
        .author(author)
        .color(data.config().embed_color)
        .description(description);

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}
