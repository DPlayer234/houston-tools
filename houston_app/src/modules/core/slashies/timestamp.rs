use std::str::FromStr;

use chrono::prelude::*;
use chrono::TimeDelta;

use crate::helper::time::parse_date_time;
use crate::slashies::prelude::*;

const DATE_TIME_INVALID: HArgError = HArgError::new_const("The time format is invalid.");
const TIME_OUT_OF_RANGE: HArgError =
    HArgError::new_const("The values are outside the allowed range.");
const SNOWFLAKE_INVALID: HArgError = HArgError::new_const("The Discord snowflake is invalid.");

/// Provides methods for localized timestamps.
#[chat_command(
    contexts = "Guild | BotDm | PrivateChannel",
    integration_types = "Guild | User"
)]
pub mod timestamp {
    /// Gets a timestamp offset from the current time.
    #[sub_command]
    async fn r#in(
        ctx: Context<'_>,
        #[description = "Days in the future."] days: Option<i64>,
        #[description = "Hours in the future."] hours: Option<i64>,
        #[description = "Minutes in the future."] minutes: Option<i64>,
    ) -> Result {
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

        show_timestamp(ctx, timestamp).await
    }

    /// Gets a timestamp at the specified time.
    #[sub_command]
    async fn at(
        ctx: Context<'_>,
        #[description = "Format is 'YYYY-MM-DD HH:mm', f.e.: '2024-03-20 15:28'"] date_time: &str,
    ) -> Result {
        let timestamp = parse_date_time(date_time, Utc).ok_or(DATE_TIME_INVALID)?;

        show_timestamp(ctx, timestamp).await
    }

    /// Gets the creation timestamp from a Discord snowflake.
    #[sub_command]
    async fn of(
        ctx: Context<'_>,
        #[description = "The Discord snowflake."] snowflake: &str,
    ) -> Result {
        let timestamp = UserId::from_str(snowflake)
            .ok()
            .map(|s| *s.created_at())
            .ok_or(SNOWFLAKE_INVALID)?;

        show_timestamp(ctx, timestamp).await
    }
}

async fn show_timestamp<Tz: TimeZone>(ctx: Context<'_>, timestamp: DateTime<Tz>) -> Result {
    fn format_time(timestamp: i64, f: char) -> String {
        format!("<t:{timestamp}:{f}>\n```\n<t:{timestamp}:{f}>\n```")
    }

    let timestamp = timestamp.timestamp();
    let embed = CreateEmbed::new()
        .field("Date & Time", format_time(timestamp, 'f'), true)
        .field("Time Only", format_time(timestamp, 't'), true)
        .field("Relative", format_time(timestamp, 'R'), true)
        .color(ctx.data_ref().config().embed_color);

    ctx.send(create_reply(Ephemeral).embed(embed)).await?;
    Ok(())
}
