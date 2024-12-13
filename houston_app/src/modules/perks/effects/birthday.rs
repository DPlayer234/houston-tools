use std::slice;

use anyhow::Context as _;
use bson::doc;
use chrono::prelude::*;
use utils::text::write_str::*;

use super::*;
use crate::fmt::replace_holes;
use crate::modules::perks::config::BirthdayGuildConfig;
use crate::modules::perks::model::{self, *};
use crate::modules::perks::DayOfYear;

pub struct Birthday;

impl Shape for Birthday {
    async fn supported(&self, args: Args<'_>) -> Result<bool> {
        // this only errors if there is no role
        Ok(get_guild_config(&args).is_ok())
    }

    async fn enable(&self, args: Args<'_>, _state: Option<Bson>) -> Result {
        let config = get_guild_config(&args)?;
        let db = args.ctx.data_ref::<HContextData>().database()?;

        log::info!("Start birthday of {} in {}.", args.user_id, args.guild_id);

        if let Some(role) = config.role {
            args.ctx
                .http
                .add_member_role(
                    args.guild_id,
                    args.user_id,
                    role,
                    Some("it's their birthday"),
                )
                .await?;
        }

        for &(item, amount) in &config.gifts {
            Wallet::collection(db)
                .add_items(args.guild_id, args.user_id, item, amount.into())
                .await?;
        }

        if let Some(notice) = &config.notice {
            let message = replace_holes(&notice.text, |out, n| match n {
                "user" => write_str!(out, "<@{}>", args.user_id),
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

        Ok(())
    }

    async fn disable(&self, args: Args<'_>) -> Result {
        if let Ok(config) = get_guild_config(&args) {
            if let Some(role) = config.role {
                args.ctx
                    .http
                    .remove_member_role(
                        args.guild_id,
                        args.user_id,
                        role,
                        Some("their birthday is over"),
                    )
                    .await?;
            }
        }

        log::info!("End birthday of {} in {}.", args.user_id, args.guild_id);

        Ok(())
    }

    async fn update(&self, ctx: &Context, now: DateTime<Utc>) -> Result {
        let data = ctx.data::<HContextData>();
        let perks = data.config().perks()?;
        let Some(birthday) = &perks.birthday else {
            return Ok(());
        };

        'regions: for (region_id, region) in birthday.regions.iter().enumerate() {
            #[allow(clippy::cast_possible_wrap)]
            #[allow(clippy::cast_possible_truncation)]
            let region_id = region_id as i32;

            // calculate the correct date with the current time and offset
            let today = now
                .checked_add_signed(region.time_offset)
                .context("birthday time offset breaks start time")?
                .naive_utc()
                .date();

            // don't repeat the check if we checked that day already
            let mut check = region.last_check.write().await;
            if *check == today {
                continue 'regions;
            }

            // from the current date, consider the offset and calculate the end time
            let tomorrow = today
                .and_time(NaiveTime::MIN)
                .and_utc()
                .checked_sub_signed(region.time_offset)
                .context("birthday time offset breaks end time")?
                .checked_add_signed(birthday.duration)
                .context("tomorrow does not exist")?;

            let db = data.database()?;

            let days = DayOfYear::search_days(today);
            log::trace!("Check: {} on {today} as {days:?}", region.name);

            let filter = doc! {
                "region": region_id,
                "day_of_year": {
                    "$in": bson::ser::to_bson(&days)?,
                },
            };

            // for all users with a birthday, try to enable the perk per guild
            let mut users = model::Birthday::collection(db).find(filter).await?;
            while let Some(user) = users.try_next().await? {
                'guild: for &guild in birthday.guilds.keys() {
                    let has_perk = ActivePerk::collection(db)
                        .find_enabled(guild, user.user, Effect::Birthday)
                        .await?
                        .is_some();

                    if has_perk {
                        continue 'guild;
                    }

                    let args = Args::new(ctx, guild, user.user);
                    self.enable(args, None).await?;

                    ActivePerk::collection(db)
                        .set_enabled(guild, user.user, Effect::Birthday, tomorrow)
                        .await?;
                }
            }

            *check = today;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("birthday rewards not configured for this guild")]
struct NoBirthday;

fn get_guild_config<'a>(args: &Args<'a>) -> Result<&'a BirthdayGuildConfig, NoBirthday> {
    args.ctx
        .data_ref::<HContextData>()
        .config()
        .perks
        .as_ref()
        .ok_or(NoBirthday)?
        .birthday
        .as_ref()
        .ok_or(NoBirthday)?
        .guilds
        .get(&args.guild_id)
        .ok_or(NoBirthday)
}
