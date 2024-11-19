use anyhow::Context as _;
use bson::doc;

use utils::text::write_str::*;

use crate::helper::bson::bson_id;
use crate::modules::perks::model::Wallet;
use crate::modules::perks::Item;
use crate::prelude::*;

/// View your server wallet.
#[poise::command(
    slash_command,
    guild_only,
    install_context = "Guild",
    interaction_context = "Guild",
)]
pub async fn wallet(
    ctx: HContext<'_>,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> HResult {
    let data = ctx.data_ref();
    let guild_id = ctx.guild_id().context("must be used in guild")?;
    let perks = data.config().perks()?;
    let db = data.database()?;

    ctx.defer_as(ephemeral).await?;

    let filter = doc! {
        "user": bson_id!(ctx.author().id),
        "guild": bson_id!(guild_id),
    };

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

    if description.is_empty() {
        "<None>".clone_into(&mut description);
    }

    let (display_name, face) = get_display_info(&ctx);
    let author = format!("{display_name}: Wallet");
    let author = CreateEmbedAuthor::new(author).icon_url(face);

    let embed = CreateEmbed::new()
        .author(author)
        .color(data.config().embed_color)
        .description(description);

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}

fn get_display_info<'a>(ctx: &HContext<'a>) -> (&'a str, String) {
    match &ctx.interaction.member {
        Some(member) => (
            member.display_name(),
            member.face(),
        ),
        _ => (
            ctx.author().display_name(),
            ctx.author().face(),
        )
    }
}
