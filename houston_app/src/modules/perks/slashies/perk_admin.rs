use anyhow::Context;
use bson::doc;
use chrono::*;

use serenity::futures::TryStreamExt;
use utils::text::write_str::*;
use utils::time::TimeMentionable;

use super::super::effects::{Args, Kind};
use crate::helper::bson_id;
use crate::modules::perks::model::*;
use crate::prelude::*;
use crate::slashies::command_group;

command_group!(
    /// Managed active perks.
    pub perk_admin (
        rename = "perk-admin",
        default_member_permissions = "MANAGE_GUILD",
        guild_only,
    ),
    "enable", "disable", "list"
);

/// Enables a perk for a member.
#[poise::command(slash_command, guild_only)]
async fn enable(
    ctx: HContext<'_>,
    #[description = "The member to enable the perk for."]
    member: Member,
    #[description = "The perk to enable."]
    perk: Kind,
    #[description = "How long to enable it for, in hours."]
    duration: u32,
) -> HResult {
    let db = ctx.data_ref().database()?;
    let args = Args {
        ctx: ctx.serenity_context(),
        guild_id: member.guild_id,
        user_id: member.user.id,
    };

    let duration = TimeDelta::try_hours(i64::from(duration))
        .context("too many hours")?;

    let until = Utc::now()
        .checked_add_signed(duration)
        .context("duration lasts beyond the end of time")?;

    ctx.defer_ephemeral().await?;
    perk.enable(args).await?;

    ActivePerk::collection(db)
        .set_enabled(member.guild_id, member.user.id, perk, until)
        .await?;

    let description = format!(
        "Enabled **{}** for {} until {}.",
        perk.name(), member.mention(), until.short_date_time(),
    );

    let embed = CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .description(description);

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}

/// Disables a perk for a member.
#[poise::command(slash_command, guild_only)]
async fn disable(
    ctx: HContext<'_>,
    #[description = "The member to disable the perk for."]
    member: Member,
    #[description = "The perk to disable."]
    perk: Kind,
) -> HResult {
    let db = ctx.data_ref().database()?;
    let args = Args {
        ctx: ctx.serenity_context(),
        guild_id: member.guild_id,
        user_id: member.user.id,
    };

    ctx.defer_ephemeral().await?;
    perk.disable(args).await?;

    ActivePerk::collection(db)
        .set_disabled(member.guild_id, member.user.id, perk)
        .await?;

    let description = format!(
        "Disabled **{}** for {}.",
        perk.name(), member.mention(),
    );

    let embed = CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .description(description);

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}

/// List active perks of a member.
#[poise::command(slash_command, guild_only)]
async fn list(
    ctx: HContext<'_>,
    #[description = "The member to check."]
    member: Member,
) -> HResult {
    let db = ctx.data_ref().database()?;
    ctx.defer_ephemeral().await?;

    let filter = doc! {
        "guild": bson_id!(member.guild_id),
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
            perk.effect.name(), perk.until.short_date_time(),
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
        .color(DEFAULT_EMBED_COLOR)
        .description(description);

    ctx.send(CreateReply::new().embed(embed)).await?;
    Ok(())
}
