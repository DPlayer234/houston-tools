use std::str::FromStr as _;

use time::UtcDateTime;

use crate::helper::time::{parse_date_time, parse_dhms_duration};
use crate::slashies::prelude::*;

/// Provides methods for localized timestamps.
#[chat_command(
    contexts = "Guild | BotDm | PrivateChannel",
    integration_types = "Guild | User"
)]
pub mod timestamp {
    use time::UtcDateTime;

    /// Gets a timestamp offset from the current time.
    #[sub_command]
    async fn r#in(
        ctx: Context<'_>,
        /// The offset from the current time, specified in "H:MM:SS" format.
        delta: &str,
    ) -> Result {
        let delta = parse_dhms_duration(delta).ok_or(HArgError::new_const(
            "Invalid duration. The expected format is `H:MM:SS`, f.e. `1:00:00` for 1 hour.",
        ))?;

        let timestamp = UtcDateTime::now()
            .checked_add(delta)
            .ok_or(HArgError::new_const("The inputs exceed the allowed range."))?
            .truncate_to_minute();

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

        let timestamp = parse_date_time(date_time).ok_or(INVALID_INPUT)?;
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
            .map_err(|_| HArgError::new_const("The Discord snowflake is invalid."))?
            .created_at()
            .to_utc();

        show_timestamp(ctx, timestamp).await
    }
}

async fn show_timestamp(ctx: Context<'_>, timestamp: UtcDateTime) -> Result {
    fn format_time(timestamp: i64, f: char) -> String {
        format!("<t:{timestamp}:{f}>\n```\n<t:{timestamp}:{f}>\n```")
    }

    let timestamp = timestamp.unix_timestamp();
    let embed = CreateEmbed::new()
        .field("Date & Time", format_time(timestamp, 'f'), true)
        .field("Time Only", format_time(timestamp, 't'), true)
        .field("Relative", format_time(timestamp, 'R'), true)
        .color(ctx.data_ref().config().embed_color);

    ctx.send(create_reply(Ephemeral).embed(embed)).await?;
    Ok(())
}
