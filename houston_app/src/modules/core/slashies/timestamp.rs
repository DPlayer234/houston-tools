use std::str::FromStr as _;

use chrono::TimeDelta;
use chrono::prelude::*;

use crate::helper::time::parse_date_time;
use crate::slashies::prelude::*;

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
        /// Days in the future.
        days: Option<i64>,
        /// Hours in the future.
        hours: Option<i64>,
        /// Minutes in the future.
        minutes: Option<i64>,
    ) -> Result {
        const TIME_OUT_OF_RANGE: HArgError =
            HArgError::new_const("The inputs exceed the allowed range.");

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
        /// The date & time in a format like '2024-04-16 14:53'.
        #[name = "date-time"]
        date_time: &str,
    ) -> Result {
        const INVALID_INPUT: HArgError = HArgError::new_const(
            "The input doesn't match any expected format.\n\
             \n\
             Here are some allowed examples, each representing the same time:\n\
             - `2024-04-16 14:53`\n\
             - `16.04.2024 14:53`\n\
             - `04/16/2024 02:53pm`\n\
             - `2024-04-16 15:53 +01`\n\
             - `2024-04-16 16:23 +01:30`\n\
             - `Apr 16, 2024 14:53`",
        );

        let timestamp = parse_date_time(date_time, Utc).ok_or(INVALID_INPUT)?;
        show_timestamp(ctx, timestamp).await
    }

    /// Gets the creation timestamp from a Discord snowflake.
    #[sub_command]
    async fn of(
        ctx: Context<'_>,
        /// The Discord snowflake.
        snowflake: &str,
    ) -> Result {
        let timestamp = UserId::from_str(snowflake)
            .ok()
            .map(|s| *s.created_at())
            .ok_or(HArgError::new_const("The Discord snowflake is invalid."))?;

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
