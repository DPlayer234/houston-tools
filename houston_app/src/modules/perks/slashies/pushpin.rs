use anyhow::Context as _;

use crate::modules::perks::items::Item;
use crate::modules::perks::model::*;
use crate::prelude::*;

#[poise::command(
    context_menu_command = "[pin/overridden]",
    guild_only,
    install_context = "Guild",
    interaction_context = "Guild",
)]
pub async fn pushpin_pin(
    ctx: HContext<'_>,
    message: Message,
) -> HResult {
    let data = ctx.data_ref();
    let guild_id = ctx.guild_id().context("must be used in guild")?;
    let perks = data.config().perks()?;
    let db = data.database()?;

    if message.pinned() {
        let embed = CreateEmbed::new()
            .color(ERROR_EMBED_COLOR)
            .description("This message is already pinned.");

        ctx.send(CreateReply::new().embed(embed).ephemeral(true)).await?;
    } else {
        ctx.defer_ephemeral().await?;

        Wallet::collection(db)
            .take_items(guild_id, ctx.author().id, Item::Pushpin, 1, perks)
            .await?;

        match message.pin(ctx.http(), Some("pinned by pushpin item")).await {
            Ok(()) => {
                let description = format!(
                    "Pinned!\n-# Used 1 {}.",
                    Item::Pushpin.info(perks).name,
                );

                let embed = CreateEmbed::new()
                    .color(data.config().embed_color)
                    .description(description);

                ctx.send(CreateReply::new().embed(embed)).await?;
            }
            Err(_) => {
                Wallet::collection(db)
                    .add_items(guild_id, ctx.author().id, Item::Pushpin, 1)
                    .await?;

                let embed = CreateEmbed::new()
                    .color(ERROR_EMBED_COLOR)
                    .description("Can't pin this.");

                ctx.send(CreateReply::new().embed(embed)).await?;
            }
        }
    }

    Ok(())
}

#[poise::command(
    context_menu_command = "[unpin/overridden]",
    guild_only,
    install_context = "Guild",
    interaction_context = "Guild",
)]
pub async fn pushpin_unpin(
    ctx: HContext<'_>,
    message: Message,
) -> HResult {
    let data = ctx.data_ref();
    let guild_id = ctx.guild_id().context("must be used in guild")?;
    let perks = data.config().perks()?;
    let db = data.database()?;

    if !message.pinned() {
        let embed = CreateEmbed::new()
            .color(ERROR_EMBED_COLOR)
            .description("This message isn't pinned.");

        ctx.send(CreateReply::new().embed(embed).ephemeral(true)).await?;
    } else {
        ctx.defer_ephemeral().await?;

        Wallet::collection(db)
            .take_items(guild_id, ctx.author().id, Item::Pushpin, 1, perks)
            .await?;

        match message.unpin(ctx.http(), Some("unpinned by pushpin item")).await {
            Ok(()) => {
                let description = format!(
                    "Unpinned!\n-# Used 1 {}.",
                    Item::Pushpin.info(perks).name,
                );

                let embed = CreateEmbed::new()
                    .color(data.config().embed_color)
                    .description(description);

                ctx.send(CreateReply::new().embed(embed)).await?;
            }
            Err(_) => {
                Wallet::collection(db)
                    .add_items(guild_id, ctx.author().id, Item::Pushpin, 1)
                    .await?;

                let embed = CreateEmbed::new()
                    .color(ERROR_EMBED_COLOR)
                    .description("Can't unpin this.");

                ctx.send(CreateReply::new().embed(embed)).await?;
            }
        }
    }

    Ok(())
}
