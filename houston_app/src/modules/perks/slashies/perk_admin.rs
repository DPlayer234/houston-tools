use anyhow::Context;
use bson::doc;
use chrono::*;

use serenity::futures::TryStreamExt;
use utils::text::write_str::*;

use crate::helper::bson_id;
use crate::fmt::discord::TimeMentionable;
use crate::modules::perks::effects::{Args, Effect};
use crate::modules::perks::items::Item;
use crate::modules::perks::model::*;
use crate::prelude::*;
use crate::slashies::args::SlashMember;
use crate::slashies::command_group;

command_group!(
    /// Managed active perks.
    pub perk_admin (
        rename = "perk-admin",
        default_member_permissions = "MANAGE_GUILD",
        guild_only,
        install_context = "Guild",
        interaction_context = "Guild",
    ),
    "enable", "disable", "list", "give", "unique_role",
);

/// Enables a perk for a member.
#[poise::command(
    slash_command,
)]
async fn enable(
    ctx: HContext<'_>,
    #[description = "The member to enable the perk for."]
    member: SlashMember,
    #[description = "The perk to enable."]
    perk: Effect,
    #[description = "How long to enable it for, in hours."]
    duration: u32,
) -> HResult {
    let data = ctx.data_ref();
    let guild_id = ctx.require_guild_id()?;
    let perks = data.config().perks()?;
    let db = data.database()?;
    let args = Args::new(ctx.serenity_context(), guild_id, member.user.id);

    let duration = TimeDelta::try_hours(i64::from(duration))
        .context("too many hours")?;

    let until = Utc::now()
        .checked_add_signed(duration)
        .context("duration lasts beyond the end of time")?;

    ctx.defer_ephemeral().await?;
    perk.enable(args, None).await?;

    ActivePerk::collection(db)
        .set_enabled(guild_id, member.user.id, perk, until)
        .await?;

    let description = format!(
        "Enabled **{}** for {} until {}.",
        perk.info(perks).name, member.mention(), until.short_date_time(),
    );

    let embed = CreateEmbed::new()
        .color(data.config().embed_color)
        .description(description);

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}

/// Disables a perk for a member.
#[poise::command(
    slash_command,
)]
async fn disable(
    ctx: HContext<'_>,
    #[description = "The member to disable the perk for."]
    member: SlashMember,
    #[description = "The perk to disable."]
    perk: Effect,
) -> HResult {
    let data = ctx.data_ref();
    let guild_id = ctx.require_guild_id()?;
    let perks = data.config().perks()?;
    let db = data.database()?;
    let args = Args::new(ctx.serenity_context(), guild_id, member.user.id);

    ctx.defer_ephemeral().await?;
    perk.disable(args).await?;

    ActivePerk::collection(db)
        .set_disabled(guild_id, member.user.id, perk)
        .await?;

    let description = format!(
        "Disabled **{}** for {}.",
        perk.info(perks).name, member.mention(),
    );

    let embed = CreateEmbed::new()
        .color(data.config().embed_color)
        .description(description);

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}

/// List active perks of a member.
#[poise::command(
    slash_command,
)]
async fn list(
    ctx: HContext<'_>,
    #[description = "The member to check."]
    member: SlashMember,
) -> HResult {
    let data = ctx.data_ref();
    let guild_id = ctx.require_guild_id()?;
    let perks = data.config().perks()?;
    let db = data.database()?;
    ctx.defer_ephemeral().await?;

    let filter = doc! {
        "guild": bson_id!(guild_id),
        "user": bson_id!(member.user.id),
    };

    let mut query = ActivePerk::collection(db)
        .find(filter)
        .await?;

    let mut description = String::new();

    while let Some(perk) = query.try_next().await? {
        writeln_str!(
            description,
            "- **{}:** Ends {}",
            perk.effect.info(perks).name, perk.until.short_date_time(),
        );
    }

    if description.is_empty() {
        "<None>".clone_into(&mut description);
    }

    let title = format!(
        "{}'s Perks",
        member.display_name(),
    );

    let embed = CreateEmbed::new()
        .title(title)
        .color(data.config().embed_color)
        .description(description);

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}

/// Gives a user items.
#[poise::command(
    slash_command,
)]
async fn give(
    ctx: HContext<'_>,
    #[description = "The member to give items to."]
    member: SlashMember,
    #[description = "The item to hand out."]
    item: Item,
    #[description = "How many items to give. Negative to remove."]
    amount: i32,
) -> HResult {
    let data = ctx.data_ref();
    let guild_id = ctx.require_guild_id()?;
    let perks = data.config().perks()?;
    let db = data.database()?;
    ctx.defer_ephemeral().await?;

    let wallet = Wallet::collection(db)
        .add_items(guild_id, member.user.id, item, amount.into())
        .await?;

    let description = format!(
        "Set **{}** to {} for {}.",
        item.info(perks).name, wallet.item(item), member.mention(),
    );

    let embed = CreateEmbed::new()
        .color(data.config().embed_color)
        .description(description);

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}

/// Sets a user's unique role. Can be omitted to delete the association.
#[poise::command(
    slash_command,
    rename = "unique-role",
)]
async fn unique_role(
    ctx: HContext<'_>,
    #[description = "The member to give items to."]
    member: SlashMember,
    #[description = "The role to set as being unique to them."]
    role: Option<Role>,
) -> HResult {
    let data = ctx.data_ref();
    let guild_id = ctx.require_guild_id()?;
    let db = data.database()?;
    ctx.defer_ephemeral().await?;

    let filter = doc! {
        "guild": bson_id!(guild_id),
        "user": bson_id!(member.user.id),
    };

    let description = if let Some(role) = role {
        let update = doc! {
            "$setOnInsert": {
                "guild": bson_id!(guild_id),
                "user": bson_id!(member.user.id),
            },
            "$set": {
                "role": bson_id!(role.id),
            },
        };

        UniqueRole::collection(db)
            .update_one(filter, update)
            .upsert(true)
            .await?;

        format!(
            "Set {}'s unique role to be {}.",
            member.mention(), role.mention(),
        )
    } else {
        UniqueRole::collection(db)
            .delete_one(filter)
            .await?;

        format!(
            "Unset {}'s unique role.",
            member.mention(),
        )
    };

    let embed = CreateEmbed::new()
        .color(data.config().embed_color)
        .description(description);

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}
