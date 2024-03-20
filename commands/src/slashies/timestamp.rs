use std::str::FromStr;
use crate::internal::prelude::*;
use chrono::prelude::*;
use chrono::TimeDelta;

/// Provides methods to help with timestamps.
#[poise::command(
    slash_command,
    subcommands("timestamp_in", "timestamp_at"),
    subcommand_required
)]
pub async fn timestamp(_: HContext<'_>) -> HResult {
    Ok(())
}

/// Returns a timestamp offset from the current time.
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

/// Returns a timestamp at the specified time.
#[poise::command(slash_command, rename = "at")]
async fn timestamp_at(
    ctx: HContext<'_>,
    #[description = "Format is 'YYYY-MM-DD HH:mm:ss Z', f.e.: '2024-03-20 15:28:00 +01:00'"]
    timestamp: String
) -> HResult {
    let timestamp: DateTime<FixedOffset> = DateTime::from_str(&timestamp).map_err(|_| DATETIME_INVALID)?;
    show_timestamp(&ctx, timestamp).await
}

async fn show_timestamp<Tz: TimeZone>(ctx: &HContext<'_>, timestamp: DateTime<Tz>) -> HResult {
    fn format_time(timestamp: i64, f: char) -> String {
        format!("<t:{timestamp}:{f}>\n```\n<t:{timestamp}:{f}>\n```")
    }
    
    let timestamp = timestamp.timestamp();
    let embed = CreateEmbed::default()
        .field("Date & Time", format_time(timestamp, 'f'), true)
        .field("Time Only", format_time(timestamp, 't'), true)
        .field("Relative", format_time(timestamp, 'R'), true)
        .color(DEFAULT_EMBED_COLOR);

    ctx.send(ctx.create_reply().embed(embed)).await?;
    Ok(())
}

const DATETIME_INVALID: HArgError = HArgError("The time format is invalid. Did you include seconds?");
const TIME_OUT_OF_RANGE: HArgError = HArgError("The values are outside the allowed range.");
