use bson::doc;

use utils::text::write_str::*;

use crate::helper::bson::bson_id;
use crate::modules::Module as _;
use crate::slashies::prelude::*;

/// View a member's server profile.
#[context_command(
    user,
    name = "Server Profile",
    contexts = "Guild",
    integration_types = "Guild",
)]
pub async fn profile_context(
    ctx: Context<'_>,
    member: SlashMember<'_>,
) -> Result {
    profile_core(ctx, member, None).await
}

/// View a member's server profile.
#[chat_command(
    contexts = "Guild",
    integration_types = "Guild",
)]
pub async fn profile(
    ctx: Context<'_>,
    #[description = "The member to view the profile of."]
    member: Option<SlashMember<'_>>,
    #[description = "Whether to show the response only to yourself."]
    ephemeral: Option<bool>,
) -> Result {
    let member = match member {
        Some(member) => member,
        None => SlashMember::from_ctx(ctx)?,
    };

    profile_core(ctx, member, ephemeral).await
}

async fn profile_core(
    ctx: Context<'_>,
    member: SlashMember<'_>,
    ephemeral: Option<bool>,
) -> Result {
    let data = ctx.data_ref();
    ctx.defer_as(ephemeral).await?;

    let author = format!("{}: Profile", member.display_name());
    let author = CreateEmbedAuthor::new(author).icon_url(member.face());

    let mut embed = CreateEmbed::new()
        .author(author)
        .color(data.config().embed_color);

    if crate::modules::starboard::Module.enabled(data.config()) {
        if let Some(starboard) = starboard_info(ctx, member).await? {
            embed = embed.field("Starboard", starboard, false);
        }
    }

    if crate::modules::perks::Module.enabled(data.config()) {
        if let Some(unique_role) = perks_unique_role(ctx, member).await? {
            embed = embed.description(format!("-# <@&{unique_role}>"));
        }

        if let Some(info) = perks_collectible_info(ctx, member).await? {
            embed = embed.field("Collection", info, false);
        }
    }

    let reply = CreateReply::new()
        .embed(embed);

    ctx.send(reply).await?;
    Ok(())
}

async fn perks_unique_role(
    ctx: Context<'_>,
    member: SlashMember<'_>,
) -> Result<Option<RoleId>> {
    use crate::modules::perks::model;

    let data = ctx.data_ref();
    let db = data.database()?;
    let guild_id = ctx.require_guild_id()?;

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
    ctx: Context<'_>,
    member: SlashMember<'_>,
) -> Result<Option<String>> {
    use crate::modules::perks::model;
    use crate::modules::perks::Item;

    let data = ctx.data_ref();
    let db = data.database()?;
    let perks = data.config().perks()?;
    let guild_id = ctx.require_guild_id()?;

    let Some(collectible) = perks.collectible.as_ref() else {
        return Ok(None);
    };

    let filter = doc! {
        "guild": bson_id!(guild_id),
        "user": bson_id!(member.user.id),
    };

    let wallet = model::Wallet::collection(db)
        .find_one(filter)
        .await?
        .unwrap_or_default();

    let mut content = format!(
        "-# **{}:** x{}",
        Item::Collectible.info(perks).name, wallet.crab,
    );

    if let Some(guild_config) = collectible.guilds.get(&guild_id) {
        for &(need, role) in &guild_config.prize_roles {
            if wallet.crab >= need.into() {
                write_str!(content, "\n- <@&{role}>");
            } else {
                write_str!(content, "\n- -# 🔒 ({need})")
            }
        }
    }

    Ok(Some(content))
}

async fn starboard_info(
    ctx: Context<'_>,
    member: SlashMember<'_>,
) -> Result<Option<String>> {
    use crate::modules::starboard::model;

    let data = ctx.data_ref();
    let db = data.database()?;
    let guild_id = ctx.require_guild_id()?;

    let guild_config = data.config()
        .starboard
        .get(&guild_id);

    let Some(guild_config) = guild_config else {
        return Ok(None);
    };

    let filter = doc! {
        "user": bson_id!(member.user.id),
        "board": {
            "$in": guild_config.board_db_keys(),
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
