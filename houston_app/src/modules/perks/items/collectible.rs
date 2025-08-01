use std::ops::{Bound, RangeBounds as _};
use std::slice;

use utils::text::WriteStr as _;

use super::*;
use crate::fmt::replace_holes;

pub struct Collectible;

impl Shape for Collectible {
    async fn on_buy(&self, args: Args<'_>, from: i64, to: i64) -> Result {
        let config = args
            .ctx
            .data_ref::<HContextData>()
            .config()
            .perks()?
            .collectible
            .as_ref()
            .context("perks.collectible must be enabled")?;

        if let Some(guild_config) = config.guilds.get(&args.guild_id) {
            let range = (Bound::Excluded(from), Bound::Included(to));
            let roles = guild_config
                .prize_roles
                .iter()
                .filter(move |e| range.contains(&i64::from(e.0)));

            for &(need, role) in roles {
                args.ctx
                    .http
                    .add_member_role(
                        args.guild_id,
                        args.user_id,
                        role,
                        Some(&format!("hit {need} collectible threshold")),
                    )
                    .await
                    .context("could not add collectible role")?;

                if let Some(notice) = &guild_config.notice {
                    let message = replace_holes(&notice.text, |out, n| match n {
                        "user" => write!(out, "{}", args.user_id.mention()),
                        "role" => write!(out, "{}", role.mention()),
                        _ => out.push(char::REPLACEMENT_CHARACTER),
                    });

                    // ping the user but _no_ roles
                    let allowed_mentions = CreateAllowedMentions::new()
                        .users(slice::from_ref(&args.user_id))
                        .empty_roles();

                    let message = CreateMessage::new()
                        .content(message)
                        .allowed_mentions(allowed_mentions);

                    notice
                        .channel
                        .send_message(&args.ctx.http, message)
                        .await
                        .context("could not send collectible role notice")?;
                }
            }
        }

        Ok(())
    }
}
