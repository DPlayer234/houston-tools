use anyhow::Context as _;
use chrono::prelude::*;

use super::*;
use crate::modules::perks::config::RainbowRoleEntry;

pub struct RainbowRole;

impl Shape for RainbowRole {
    async fn supported(&self, args: Args<'_>) -> anyhow::Result<bool> {
        // this only errors if there is no role
        Ok(find_rainbow_role(&args).is_ok())
    }

    async fn enable(&self, args: Args<'_>) -> HResult {
        let role = find_rainbow_role(&args)?;

        args.ctx.http.add_member_role(
            args.guild_id,
            args.user_id,
            role.role,
            Some("enabled rainbow role perk"),
        ).await?;
        Ok(())
    }

    async fn disable(&self, args: Args<'_>) -> HResult {
        let role = find_rainbow_role(&args)?;

        args.ctx.http.remove_member_role(
            args.guild_id,
            args.user_id,
            role.role,
            Some("disabled rainbow role perk"),
        ).await?;
        Ok(())
    }

    async fn update(&self, ctx: &Context) -> HResult {
        const LOOP_TIME: i64 = 7200;

        let loop_sec = Utc::now()
            .time()
            .signed_duration_since(NaiveTime::MIN)
            .num_seconds() % LOOP_TIME;

        let loop_rel = loop_sec as f64 / LOOP_TIME as f64;

        let h = loop_rel * 360.0;
        let s = match h {
            220.0..240.0 => 1.0 - (h - 220.0) / 100.0,
            240.0..260.0 => 0.8,
            260.0..280.0 => 1.0 - (280.0 - h) / 100.0,
            _ => 1.0,
        };
        let v = 1.0;

        let color = hsv_to_color(h, s, v);

        for entry in rainbow_roles(ctx) {
            let edit = EditRole::new()
                .colour(color);

            let role = entry.guild.edit_role(&ctx.http, entry.role, edit).await?;
            log::trace!("Updated rainbow role {} to color #{:06X}", role.name, color.0);
        }

        Ok(())
    }
}

fn rainbow_roles(ctx: &Context) -> &[RainbowRoleEntry] {
    ctx.data_ref::<HBotData>()
        .config()
        .perks.as_ref()
        .expect("must have perks enabled")
        .rainbow.as_ref()
        .map(|r| r.role.as_slice())
        .unwrap_or_default()
}

fn find_rainbow_role<'a>(args: &Args<'a>) -> anyhow::Result<&'a RainbowRoleEntry> {
    rainbow_roles(args.ctx)
        .iter()
        .find(|r| r.guild == args.guild_id)
        .context("rainbow role not configured for guild")
}

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
fn rgb(r: f64, g: f64, b: f64) -> Color {
    Color::from_rgb(
        (r * 255.0).clamp(0.0, 255.0) as u8,
        (g * 255.0).clamp(0.0, 255.0) as u8,
        (b * 255.0).clamp(0.0, 255.0) as u8,
    )
}

fn hsv_to_color(mut h: f64, s: f64, v: f64) -> Color {
	h = ((h % 360.0) + 360.0) % 360.0;

	let mut c = v * s;
	let mut x = c * (1.0 - f64::abs((h / 60.0) % 2.0 - 1.0));
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
