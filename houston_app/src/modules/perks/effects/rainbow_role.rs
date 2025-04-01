use anyhow::Context as _;
use bson_model::ModelDocument as _;
use chrono::prelude::*;

use super::*;
use crate::modules::perks::config::{RainbowConfig, RainbowRoleEntry};
use crate::modules::perks::model::*;

pub struct RainbowRole;

impl Shape for RainbowRole {
    async fn supported(&self, args: Args<'_>) -> Result<bool> {
        // this only errors if there is no role
        Ok(find_rainbow_role(&args).is_ok())
    }

    async fn enable(&self, args: Args<'_>, _state: Option<Bson>) -> Result {
        let role = find_rainbow_role(&args)?;

        args.ctx
            .http
            .add_member_role(
                args.guild_id,
                args.user_id,
                role.role,
                Some("enabled rainbow role perk"),
            )
            .await
            .context("could not add rainbow role")?;

        Ok(())
    }

    async fn disable(&self, args: Args<'_>) -> Result {
        if let Ok(role) = find_rainbow_role(&args) {
            let result = args
                .ctx
                .http
                .remove_member_role(
                    args.guild_id,
                    args.user_id,
                    role.role,
                    Some("disabled rainbow role perk"),
                )
                .await;

            super::ok_allowed_discord_error(result).context("could not remove rainbow role")?;
        }

        Ok(())
    }

    async fn update(&self, ctx: &Context, _now: DateTime<Utc>) -> Result {
        const LOOP_TIME: i64 = 2400;

        let Ok(rainbow) = get_config(ctx) else {
            return Ok(());
        };

        let loop_sec = Utc::now()
            .time()
            .signed_duration_since(NaiveTime::MIN)
            .num_seconds()
            .rem_euclid(LOOP_TIME);

        let loop_rel = loop_sec as f32 / LOOP_TIME as f32;

        let h = loop_rel * 360.0;
        let s = match h {
            220.0..240.0 => 1.0 - (h - 220.0) / 100.0,
            240.0..260.0 => 0.8,
            260.0..280.0 => 1.0 - (280.0 - h) / 100.0,
            _ => 1.0,
        };
        let v = 1.0;

        let color = hsv_to_color(h, s, v);

        for (&guild, entry) in &rainbow.guilds {
            if has_any_rainbow_role(ctx, guild).await? {
                let edit = EditRole::new()
                    .colour(color)
                    .audit_log_reason("rainbow role cycle");

                let role = guild
                    .edit_role(&ctx.http, entry.role, edit)
                    .await
                    .context("could not update rainbow role color")?;

                log::trace!(
                    "Updated rainbow role {} to color #{:06X}",
                    role.name,
                    color.0
                );
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("rainbow role not configured")]
struct NoRainbowRole;

fn get_config(ctx: &Context) -> Result<&RainbowConfig, NoRainbowRole> {
    ctx.data_ref::<HContextData>()
        .config()
        .perks
        .as_ref()
        .ok_or(NoRainbowRole)?
        .rainbow
        .as_ref()
        .ok_or(NoRainbowRole)
}

fn find_rainbow_role<'a>(args: &Args<'a>) -> Result<&'a RainbowRoleEntry, NoRainbowRole> {
    get_config(args.ctx)?
        .guilds
        .get(&args.guild_id)
        .ok_or(NoRainbowRole)
}

async fn has_any_rainbow_role(ctx: &Context, guild_id: GuildId) -> Result<bool> {
    let db = ctx.data_ref::<HContextData>().database()?;

    let filter = ActivePerk::filter()
        .guild(guild_id)
        .effect(Effect::RainbowRole)
        .into_document()?;

    let exists = ActivePerk::collection(db)
        .find_one(filter)
        .await
        .context("failed to check whether a rainbow role is active")?
        .is_some();

    Ok(exists)
}

#[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color::from_rgb(
        (r * 255.0).clamp(0.0, 255.0) as u8,
        (g * 255.0).clamp(0.0, 255.0) as u8,
        (b * 255.0).clamp(0.0, 255.0) as u8,
    )
}

fn hsv_to_color(mut h: f32, s: f32, v: f32) -> Color {
    h = h.rem_euclid(360.0);

    let mut c = v * s;
    let mut x = c * (1.0 - f32::abs((h / 60.0) % 2.0 - 1.0));
    let m = v - c;

    c += m;
    x += m;

    match h {
        ..60.0 => rgb(c, x, m),
        ..120.0 => rgb(x, c, m),
        ..180.0 => rgb(m, c, x),
        ..240.0 => rgb(m, x, c),
        ..300.0 => rgb(x, m, c),
        _ => rgb(c, m, x),
    }
}
