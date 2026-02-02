use std::slice;

use anyhow::Context as _;
use bson_model::{Filter, ModelDocument as _};
use time::{Time, UtcDateTime};
use utils::text::WriteStr as _;

use super::*;
use crate::fmt::replace_holes;
use crate::modules::perks::DayOfYear;
use crate::modules::perks::config::BirthdayGuildConfig;
use crate::modules::perks::model::{self, *};

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
                .await
                .context("could not add birthday role")?;
        }

        // add the gifts
        Wallet::collection(db)
            .add_items(args.guild_id, args.user_id, &config.gifts)
            .await?;

        if let Some(notice) = &config.notice {
            let message = replace_holes(&notice.text, |out, n| match n {
                "user" => write!(out, "{}", args.user_id.mention()),
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
                .context("could not send birthday notice")?;
        }

        Ok(())
    }

    async fn disable(&self, args: Args<'_>) -> Result {
        if let Ok(config) = get_guild_config(&args)
            && let Some(role) = config.role
        {
            let result = args
                .ctx
                .http
                .remove_member_role(
                    args.guild_id,
                    args.user_id,
                    role,
                    Some("their birthday is over"),
                )
                .await;

            super::ok_allowed_discord_error(result).context("could not remove birthday role")?;
        }

        log::info!("End birthday of {} in {}.", args.user_id, args.guild_id);
        Ok(())
    }

    async fn update(&self, ctx: &Context, now: UtcDateTime) -> Result {
        let data = ctx.data::<HContextData>();
        let perks = data.config().perks()?;
        let Some(birthday) = &perks.birthday else {
            return Ok(());
        };

        debug_assert!(
            u16::try_from(birthday.regions.len()).is_ok(),
            "startup validate ensures len does not overflow u16"
        );

        'regions: for (region_id, region) in (0u16..).zip(&birthday.regions) {
            // exit here if the lock is already held since that means another task is
            // handling the checks below currently. that shouldn't actually happen in
            // practice, but there is no reason to treat it as impossible.
            let Ok(mut check) = region.last_check.try_lock() else {
                continue 'regions;
            };

            // calculate the correct date with the current time and offset
            let today = now
                .checked_add(region.time_offset)
                .context("birthday time offset breaks start time")?
                .date();

            // don't repeat the check if we checked that day already
            if *check == today {
                continue 'regions;
            }

            // from the current date, consider the offset and calculate the end time
            let tomorrow = today
                .with_time(Time::MIDNIGHT)
                .checked_add(region.time_offset)
                .context("birthday time offset breaks end time")?
                .checked_add(birthday.duration)
                .context("birthday duration breaks end time")?
                .as_utc();

            let db = data.database()?;

            let days = DayOfYear::search_days(today);
            log::trace!("Check: {} on {today} as {days:?}", region.name);

            let filter = model::Birthday::filter()
                .region(region_id)
                .day_of_year(Filter::in_(days))
                .into_document()?;

            // for all users with a birthday, try to enable the perk per guild
            let mut user_entries = model::Birthday::collection(db)
                .find(filter)
                .await
                .context("failed to begin birthday query")?;

            while let Some(user_entry) = user_entries.next().await {
                let user_entry = user_entry.context("failed to get next birthday")?;
                let user = user_entry.user;

                'guild: for &guild in birthday.guilds.keys() {
                    let has_perk = ActivePerk::collection(db)
                        .find_enabled(guild, user, Effect::Birthday)
                        .await?
                        .is_some();

                    if has_perk {
                        continue 'guild;
                    }

                    // size of the `enable` future blows up the size `check_perks`
                    // so it is boxed here since it's also rarely reached
                    let args = Args::new(ctx, guild, user);
                    let result = Box::pin(self.enable(args, None)).await;

                    if super::is_known_member(result)? {
                        ActivePerk::collection(db)
                            .set_enabled(guild, user, Effect::Birthday, tomorrow)
                            .await?;
                    } else {
                        log::trace!("User {user} not in {guild}");
                    }
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
