use std::slice;

use utils::text::write_str::*;

use crate::fmt::replace_holes;

use super::*;

pub struct Collectible;

impl Shape for Collectible {
    async fn on_buy(&self, args: Args<'_>, owned: i64) -> Result {
        let config = args.ctx
            .data_ref::<HContextData>()
            .config()
            .perks()?
            .collectible.as_ref()
            .context("expected collectible config")?;

        if let Some(guild_config) = config.guilds.get(&args.guild_id) {
            let start = owned - i64::from(config.price.amount) + 1;
            let roles = guild_config.prize_roles
                .iter()
                .filter(|e| (start..=owned).contains(&e.0.into()));

            for &(need, role) in roles {
                args.ctx.http.add_member_role(
                    args.guild_id,
                    args.user_id,
                    role,
                    Some(&format!("hit {need} collectible threshold")),
                ).await?;

                if let Some(notice) = &guild_config.notice {
                    let message = replace_holes(&notice.text, |out, n| match n {
                        "user" => write_str!(out, "<@{}>", args.user_id),
                        "role" => write_str!(out, "<@&{}>", role),
                        _ => out.push(char::REPLACEMENT_CHARACTER),
                    });

                    // ping the user but _no_ roles
                    let allowed_mentions = CreateAllowedMentions::new()
                        .users(slice::from_ref(&args.user_id))
                        .empty_roles();

                    let message = CreateMessage::new()
                        .content(message)
                        .allowed_mentions(allowed_mentions);

                    notice.channel.send_message(&args.ctx.http, message).await?;
                }
            }
        }

        Ok(())
    }
}
