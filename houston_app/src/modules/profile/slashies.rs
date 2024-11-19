use anyhow::Context;
use bson::doc;
use serenity::futures::TryStreamExt;

use utils::text::write_str::*;

use crate::helper::bson::bson_id;
use crate::modules::Module as _;
use crate::prelude::*;
use crate::slashies::args::SlashMember;

/// View a member's server profile.
#[poise::command(
    context_menu_command = "Server Profile",
    guild_only,
    install_context = "Guild",
    interaction_context = "Guild",
)]
pub async fn profile_context(
    ctx: HContext<'_>,
    #[description = "The member to view the profile of."]
    member: User,
) -> HResult {
    let member = SlashMember::from_resolved(ctx, member)?;
    profile_core(ctx, member, None).await
}

/// View a member's server profile.
#[poise::command(
    slash_command,
    guild_only,
    install_context = "Guild",
    interaction_context = "Guild",
)]
pub async fn profile(
    ctx: HContext<'_>,
    #[description = "The member to view the profile of."]
    member: SlashMember,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> HResult {
    profile_core(ctx, member, ephemeral).await
}

async fn profile_core(
    ctx: HContext<'_>,
    member: SlashMember,
    ephemeral: Option<bool>,
) -> HResult {
    let data = ctx.data_ref();
    ctx.defer_as(ephemeral).await?;

    let author = format!("{}: Profile", member.display_name());
    let author = CreateEmbedAuthor::new(author).icon_url(member.face());

    let mut embed = CreateEmbed::new()
        .author(author)
        .color(data.config().embed_color);

    if crate::modules::perks::Module.enabled(data.config()) {
        if let Some(unique_role) = perks_unique_role(ctx, &member).await? {
            embed = embed.description(format!("-# <@&{unique_role}>"));
        }

        if let Some(collection) = perks_collectible_info(ctx, &member).await? {
            embed = embed.field(
                "Collection",
                collection,
                false,
            );
        }
    }

    if crate::modules::starboard::Module.enabled(data.config()) {
        if let Some(starboard) = starboard_info(ctx, &member).await? {
            embed = embed.field(
                "Starboard",
                starboard,
                false,
            );
        }
    }

    let reply = CreateReply::new()
        .embed(embed);

    ctx.send(reply).await?;
    Ok(())
}

async fn perks_unique_role(
    ctx: HContext<'_>,
    member: &SlashMember,
) -> anyhow::Result<Option<RoleId>> {
    use crate::modules::perks::model;

    let data = ctx.data_ref();
    let db = data.database()?;
    let guild_id = ctx.guild_id().context("must be used in guild")?;

    let filter = doc! {
        "guild": bson_id!(guild_id),
        "user": bson_id!(member.user.id),
    };

    let unique_role = model::UniqueRole::collection(db)
        .find_one(filter)
        .await?;

    let Some(unique_role) = unique_role else {
        return Ok(None);
    };

    Ok(Some(unique_role.role))
}

async fn perks_collectible_info(
    ctx: HContext<'_>,
    member: &SlashMember,
) -> anyhow::Result<Option<String>> {
    use crate::modules::perks::model;
    use crate::modules::perks::Item;

    let data = ctx.data_ref();
    let db = data.database()?;
    let perks = data.config().perks()?;
    let guild_id = ctx.guild_id().context("must be used in guild")?;

    let filter = doc! {
        "guild": bson_id!(guild_id),
        "user": bson_id!(member.user.id),
    };

    let wallet = model::Wallet::collection(db)
        .find_one(filter)
        .await?
        .unwrap_or_default();

    Ok((wallet.crab != 0).then(|| format!(
        "- **{}:** x{}",
        Item::Collectible.info(perks).name, wallet.crab,
    )))
}

async fn starboard_info(
    ctx: HContext<'_>,
    member: &SlashMember,
) -> anyhow::Result<Option<String>> {
    use crate::modules::starboard::model;

    let data = ctx.data_ref();
    let db = data.database()?;
    let guild_id = ctx.guild_id().context("must be used in guild")?;

    let guild_config = data.config()
        .starboard
        .get(&guild_id);

    let Some(guild_config) = guild_config else {
        return Ok(None);
    };

    let board_ids: Vec<_> = guild_config
        .boards
        .keys()
        .map(|b| b.get())
        .collect();

    let filter = doc! {
        "user": bson_id!(member.user.id),
        "board": {
            "$in": board_ids,
        },
    };

    let mut query = model::Score::collection(db)
        .find(filter)
        .await?;

    let mut description = String::new();

    while let Some(entry) = query.try_next().await? {
        let board = guild_config
            .boards
            .get(&entry.board)
            .context("board not found in config")?;

        writeln_str!(
            description,
            "- {} {} from {} post(s)",
            entry.score, board.emoji, entry.post_count,
        );
    }

    Ok((!description.is_empty()).then_some(description))
}
