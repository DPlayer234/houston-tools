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
                    let message = notice.text
                        .replace("{user}", &args.user_id.mention().to_string())
                        .replace("{role}", &role.mention().to_string());

                    let allowed_mentions = CreateAllowedMentions::new()
                        .empty_roles()
                        .all_roles(true);

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
