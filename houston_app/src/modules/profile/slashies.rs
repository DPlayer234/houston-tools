use bson_model::Filter;
use utils::text::write_str::*;

use crate::modules::Module as _;
use crate::modules::perks::DayOfYear;
use crate::slashies::prelude::*;

/// View a member's server profile.
#[context_command(
    user,
    name = "Server Profile",
    contexts = "Guild",
    integration_types = "Guild"
)]
pub async fn profile_context(ctx: Context<'_>, member: SlashMember<'_>) -> Result {
    profile_core(ctx, member, None).await
}

/// View a member's server profile.
#[chat_command(contexts = "Guild", integration_types = "Guild")]
pub async fn profile<'ctx>(
    ctx: Context<'ctx>,
    /// The member to view the profile of.
    member: Option<SlashMember<'ctx>>,
    /// Whether to show the response only to yourself.
    ephemeral: Option<bool>,
) -> Result {
    let member = member.or_invoking(ctx)?;
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

    let mut description = String::new();

    if crate::modules::starboard::Module.enabled(data.config()) {
        if let Some(starboard) = starboard_info(ctx, member).await? {
            embed = embed.field("Starboard", starboard, true);
        }
    }

    if crate::modules::perks::Module.enabled(data.config()) {
        if let Some(unique_role) = perks_unique_role(ctx, member).await? {
            writeln_str!(description, "-# <@&{unique_role}>");
        }

        if let Some(birthday) = perks_birthday(ctx, member).await? {
            writeln_str!(description, "-# **Birthday:** {birthday}");
        }

        if let Some(info) = perks_collectible_info(ctx, member).await? {
            embed = embed.field("Collection", info, true);
        }
    }

    if crate::modules::rep::Module.enabled(data.config()) {
        let rep = rep_amount(ctx, member).await?;
        writeln_str!(description, "-# **Reputation:** {rep}");
    }

    embed = embed.description(description);
    let reply = CreateReply::new().embed(embed);

    ctx.send(reply).await?;
    Ok(())
}

async fn perks_unique_role(ctx: Context<'_>, member: SlashMember<'_>) -> Result<Option<RoleId>> {
    use crate::modules::perks::model;

    let data = ctx.data_ref();
    let perks = data.config().perks()?;

    if perks.role_edit.is_none() {
        return Ok(None);
    }

    let db = data.database()?;
    let guild_id = ctx.require_guild_id()?;

    let filter = model::UniqueRole::filter()
        .guild(guild_id)
        .user(member.user.id)
        .into_document()?;

    let unique_role = model::UniqueRole::collection(db).find_one(filter).await?;

    let Some(unique_role) = unique_role else {
        return Ok(None);
    };

    Ok(Some(unique_role.role))
}

async fn perks_birthday(ctx: Context<'_>, member: SlashMember<'_>) -> Result<Option<DayOfYear>> {
    use crate::modules::perks::model;

    let data = ctx.data_ref();
    let perks = data.config().perks()?;

    if perks.birthday.is_none() {
        return Ok(None);
    }

    let db = data.database()?;

    let filter = model::Birthday::filter()
        .user(member.user.id)
        .into_document()?;

    let birthday = model::Birthday::collection(db).find_one(filter).await?;

    Ok(birthday.map(|b| b.day_of_year))
}

async fn perks_collectible_info(
    ctx: Context<'_>,
    member: SlashMember<'_>,
) -> Result<Option<String>> {
    use crate::modules::perks::{Item, model};

    let data = ctx.data_ref();
    let perks = data.config().perks()?;

    let Some(collectible) = perks.collectible.as_ref() else {
        return Ok(None);
    };

    let db = data.database()?;
    let guild_id = ctx.require_guild_id()?;

    let filter = model::Wallet::filter()
        .guild(guild_id)
        .user(member.user.id)
        .into_document()?;

    let wallet = model::Wallet::collection(db)
        .find_one(filter)
        .await?
        .unwrap_or_default();

    let mut content = format!(
        "-# **{}:** x{}",
        Item::Collectible.info(perks).name,
        wallet.crab,
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

async fn starboard_info(ctx: Context<'_>, member: SlashMember<'_>) -> Result<Option<String>> {
    use crate::modules::starboard::model;

    let data = ctx.data_ref();
    let db = data.database()?;
    let guild_id = ctx.require_guild_id()?;

    let guild_config = data.config().starboard.get(&guild_id);

    let Some(guild_config) = guild_config else {
        return Ok(None);
    };

    let filter = model::Score::filter()
        .user(member.user.id)
        .board(Filter::in_(guild_config.boards.keys().copied()))
        .into_document()?;

    let mut query = model::Score::collection(db).find(filter).await?;

    let mut description = String::new();

    while let Some(entry) = query.try_next().await? {
        let board = guild_config
            .boards
            .get(&entry.board)
            .context("board not found in config")?;

        writeln_str!(
            description,
            "- {} {} from {} post(s)",
            entry.score,
            board.emoji,
            entry.post_count,
        );
    }

    Ok((!description.is_empty()).then_some(description))
}

async fn rep_amount(ctx: Context<'_>, member: SlashMember<'_>) -> Result<i64> {
    use crate::modules::rep::model;

    let data = ctx.data_ref();
    let db = data.database()?;
    let guild_id = ctx.require_guild_id()?;

    let filter = model::Record::filter()
        .user(member.user.id)
        .guild(guild_id)
        .into_document()?;

    let rep = model::Record::collection(db)
        .find_one(filter)
        .await?
        .map(|r| r.received)
        .unwrap_or_default();

    Ok(rep)
}
