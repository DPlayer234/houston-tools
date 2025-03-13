//! Provides utilities for formatting Discord data.

use std::fmt::{Display, Formatter, Result};

use chrono::prelude::*;
use houston_cmd::ResolvedOption;

use crate::prelude::*;

/// Gets a unique username for this user.
///
/// This will either be the pomelo username or include the discriminator.
#[must_use]
pub fn get_unique_username(user: &User) -> Cow<'_, str> {
    user.discriminator
        .map(|d| format!("{}#{:04}", user.name, d).into())
        .unwrap_or_else(|| user.name.as_str().into())
}

/// Escapes markdown sequences.
#[must_use]
pub fn escape_markdown(input: &str) -> impl Display + '_ {
    utils::text::escape_by_char(input, |c| {
        matches!(c, '*' | '`' | '_' | '>').then_some(['\\', c])
    })
}

/// Allows mentioning a timestamp in Discord messages.
#[allow(dead_code, reason = "include all supported formats upfront")]
pub trait TimeMentionable {
    /// Formats a mention for a timestamp.
    fn mention(&self, format: &'static str) -> TimeMention;

    /// Formats a mention with the short time (t) format.
    fn short_time(&self) -> TimeMention {
        self.mention("t")
    }
    /// Formats a mention with the long time (T) format.
    fn long_time(&self) -> TimeMention {
        self.mention("T")
    }
    /// Formats a mention with the short date (d) format.
    fn short_date(&self) -> TimeMention {
        self.mention("d")
    }
    /// Formats a mention with the long date (D) format.
    fn long_date(&self) -> TimeMention {
        self.mention("D")
    }
    /// Formats a mention with the short date time (f) format.
    fn short_date_time(&self) -> TimeMention {
        self.mention("f")
    }
    /// Formats a mention with the long date time (F) format.
    fn long_date_time(&self) -> TimeMention {
        self.mention("F")
    }
    /// Formats a mention with the relative (R) format.
    fn relative(&self) -> TimeMention {
        self.mention("R")
    }
}

impl<Tz: TimeZone> TimeMentionable for DateTime<Tz> {
    fn mention(&self, format: &'static str) -> TimeMention {
        TimeMention {
            timestamp: self.timestamp(),
            format,
        }
    }
}

/// Formattable mention for a date and/or time.
#[derive(Debug, Clone)]
#[must_use]
pub struct TimeMention {
    timestamp: i64,
    format: &'static str,
}

impl Display for TimeMention {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "<t:{}:{}>", self.timestamp, self.format)
    }
}

/// Formattable message link.
///
/// The alternate format only prints the tail of the link.
#[derive(Debug, Clone, Copy)]
pub struct MessageLink {
    guild_id: Option<GuildId>,
    channel_id: ChannelId,
    message_id: MessageId,
}

impl MessageLink {
    /// Creates a new link from the components.
    pub fn new(
        guild_id: impl Into<Option<GuildId>>,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> Self {
        Self {
            guild_id: guild_id.into(),
            channel_id,
            message_id,
        }
    }

    /// Sets the guild ID.
    pub fn guild_id(mut self, guild_id: impl Into<Option<GuildId>>) -> Self {
        self.guild_id = guild_id.into();
        self
    }
}

impl From<&Message> for MessageLink {
    /// Creates a message link to the given message.
    ///
    /// This might be lacking the guild ID, so use [`Self::guild_id`] if you
    /// have that at hand.
    fn from(value: &Message) -> Self {
        Self::new(value.guild_id, value.channel_id, value.id)
    }
}

impl Display for MessageLink {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if !f.alternate() {
            f.write_str("https://discord.com/channels/")?;
        }

        match self.guild_id {
            Some(guild_id) => write!(f, "{guild_id}/")?,
            None => f.write_str("@me/")?,
        }

        write!(f, "{}/{}", self.channel_id, self.message_id)
    }
}

/// Implements [`Display`] to format resolved command arguments.
#[must_use]
pub enum DisplayResolvedArgs<'a> {
    /// Uses resolved options from a slash command.
    Options(&'a [ResolvedOption<'a>]),
    /// Uses the resolved target from a context menu command.
    Target(ResolvedTarget<'a>),
}

impl Display for DisplayResolvedArgs<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Options(o) => fmt_resolved_options(o, f),
            Self::Target(t) => fmt_resolved_target(t, f),
        }
    }
}

fn fmt_resolved_options(options: &[ResolvedOption<'_>], f: &mut Formatter<'_>) -> Result {
    for o in options {
        f.write_str(o.name)?;
        f.write_str(": ")?;
        fmt_resolved_option(o, f)?;
        f.write_str(" ")?;
    }

    Ok(())
}

fn fmt_resolved_option(option: &ResolvedOption<'_>, f: &mut Formatter<'_>) -> Result {
    match option.value {
        ResolvedValue::Boolean(v) => v.fmt(f),
        ResolvedValue::Integer(v) => v.fmt(f),
        ResolvedValue::Number(v) => v.fmt(f),
        ResolvedValue::String(v) => write!(f, "\"{v}\""),
        ResolvedValue::Attachment(v) => f.write_str(&v.filename),
        ResolvedValue::Channel(v) => match &v.name {
            Some(name) => f.write_str(name),
            None => v.id.fmt(f),
        },
        ResolvedValue::Role(v) => f.write_str(&v.name),
        ResolvedValue::User(v, _) => f.write_str(&v.name),
        _ => f.write_str("<unknown>"),
    }
}

fn fmt_resolved_target(target: &ResolvedTarget<'_>, f: &mut Formatter<'_>) -> Result {
    match target {
        ResolvedTarget::User(v, _) => f.write_str(&v.name),
        ResolvedTarget::Message(v) => v.id.fmt(f),
        _ => f.write_str("<unknown>"),
    }
}

/// Implements [`Display`] to format the full command name.
#[must_use]
pub struct DisplayCommandName<'a> {
    name: &'a str,
    options: &'a [CommandDataOption],
}

impl<'a> From<&'a CommandData> for DisplayCommandName<'a> {
    fn from(value: &'a CommandData) -> Self {
        Self {
            name: &value.name,
            options: &value.options,
        }
    }
}

impl Display for DisplayCommandName<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(self.name)?;
        let mut options = self.options;
        while let Some(CommandDataOption {
            name,
            value:
                CommandDataOptionValue::SubCommand(next_options)
                | CommandDataOptionValue::SubCommandGroup(next_options),
            ..
        }) = options.first()
        {
            f.write_str(" ")?;
            f.write_str(name)?;
            options = next_options;
        }

        Ok(())
    }
}
