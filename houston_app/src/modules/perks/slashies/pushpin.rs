use crate::modules::perks::items::Item;
use crate::modules::perks::model::*;
use crate::slashies::prelude::*;

/// Pin this message.
#[context_command(
    message,
    name = "[pin/overridden]",
    contexts = "Guild",
    integration_types = "Guild",
)]
pub async fn pushpin_pin(
    ctx: Context<'_>,
    message: &Message,
) -> Result {
    let data = ctx.data_ref();
    let guild_id = ctx.require_guild_id()?;
    let perks = data.config().perks()?;
    let db = data.database()?;

    if message.pinned() {
        let embed = CreateEmbed::new()
            .color(ERROR_EMBED_COLOR)
            .description("This message is already pinned.");

        ctx.send(CreateReply::new().embed(embed).ephemeral(true)).await?;
    } else {
        ctx.defer_as(Ephemeral).await?;

        Wallet::collection(db)
            .take_items(guild_id, ctx.user().id, Item::Pushpin, 1, perks)
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
                    .add_items(guild_id, ctx.user().id, Item::Pushpin, 1)
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

/// Unpin this message.
#[context_command(
    message,
    name = "[unpin/overridden]",
    contexts = "Guild",
    integration_types = "Guild",
)]
pub async fn pushpin_unpin(
    ctx: Context<'_>,
    message: &Message,
) -> Result {
    let data = ctx.data_ref();
    let guild_id = ctx.require_guild_id()?;
    let perks = data.config().perks()?;
    let db = data.database()?;

    if !message.pinned() {
        let embed = CreateEmbed::new()
            .color(ERROR_EMBED_COLOR)
            .description("This message isn't pinned.");

        ctx.send(CreateReply::new().embed(embed).ephemeral(true)).await?;
    } else {
        ctx.defer_as(Ephemeral).await?;

        Wallet::collection(db)
            .take_items(guild_id, ctx.user().id, Item::Pushpin, 1, perks)
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
                    .add_items(guild_id, ctx.user().id, Item::Pushpin, 1)
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
