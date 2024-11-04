use std::str::FromStr;

use chrono::prelude::*;
use chrono::TimeDelta;

use utils::time::{get_creation_time, parse_date_time};

use crate::prelude::*;

const DATE_TIME_INVALID: HArgError = HArgError("The time format is invalid.");
const TIME_OUT_OF_RANGE: HArgError = HArgError("The values are outside the allowed range.");
const SNOWFLAKE_INVALID: HArgError = HArgError("The Discord snowflake is invalid.");

super::command_group!(
    /// Provides methods for localized timestamps.
    pub timestamp,
    "timestamp_in", "timestamp_at", "timestamp_of"
);

/// Gets a timestamp offset from the current time.
#[poise::command(slash_command, rename = "in")]
async fn timestamp_in(
    ctx: HContext<'_>,
    #[description = "Days in the future."]
    days: Option<i64>,
    #[description = "Hours in the future."]
    hours: Option<i64>,
    #[description = "Minutes in the future."]
    minutes: Option<i64>
) -> HResult {
    let mut delta = TimeDelta::zero();

    if let Some(days) = days {
        delta += TimeDelta::try_days(days).ok_or(TIME_OUT_OF_RANGE)?;
    }

    if let Some(hours) = hours {
        delta += TimeDelta::try_hours(hours).ok_or(TIME_OUT_OF_RANGE)?;
    }

    if let Some(minutes) = minutes {
        delta += TimeDelta::try_minutes(minutes).ok_or(TIME_OUT_OF_RANGE)?;
    }

    let timestamp = Utc::now()
        .checked_add_signed(delta)
        .and_then(|d| d.with_second(0))
        .ok_or(TIME_OUT_OF_RANGE)?;

    show_timestamp(&ctx, timestamp).await
}

/// Gets a timestamp at the specified time.
#[poise::command(slash_command, rename = "at")]
async fn timestamp_at(
    ctx: HContext<'_>,
    #[description = "Format is 'YYYY-MM-DD HH:mm', f.e.: '2024-03-20 15:28'"]
    date_time: String
) -> HResult {
    let timestamp = parse_date_time(&date_time, Utc)
        .ok_or(DATE_TIME_INVALID)?;

    show_timestamp(&ctx, timestamp).await
}

/// Gets the creation timestamp from a Discord snowflake.
#[poise::command(slash_command, rename = "of")]
async fn timestamp_of(
    ctx: HContext<'_>,
    #[description = "The Discord snowflake."]
    snowflake: String
) -> HResult {
    let timestamp = u64::from_str(&snowflake).ok()
        .and_then(get_creation_time)
        .ok_or(SNOWFLAKE_INVALID)?;

    show_timestamp(&ctx, timestamp).await
}

async fn show_timestamp<Tz: TimeZone>(ctx: &HContext<'_>, timestamp: DateTime<Tz>) -> HResult {
    fn format_time(timestamp: i64, f: char) -> String {
        format!("<t:{timestamp}:{f}>\n```\n<t:{timestamp}:{f}>\n```")
    }

    let timestamp = timestamp.timestamp();
    let embed = CreateEmbed::new()
        .field("Date & Time", format_time(timestamp, 'f'), true)
        .field("Time Only", format_time(timestamp, 't'), true)
        .field("Relative", format_time(timestamp, 'R'), true)
        .color(DEFAULT_EMBED_COLOR);

    ctx.send(ctx.create_reply().embed(embed)).await?;
    Ok(())
}
