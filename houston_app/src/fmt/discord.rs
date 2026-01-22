//! Provides utilities for formatting Discord data.

use std::fmt::{Display, Formatter, Result, from_fn};

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

#[must_use]
pub fn id_suffix(id: impl Into<u64>) -> impl Display {
    let id: u64 = id.into();
    let id = (id % 10_000_000) as u32;
    utils::format_owned!("..{id:07}")
}

#[must_use]
pub fn interaction_location(
    guild_id: Option<GuildId>,
    channel: Option<&GenericInteractionChannel>,
) -> impl Display + '_ {
    from_fn(move |f| {
        match (
            guild_id.map(id_suffix),
            channel.and_then(|c| c.base().name.as_deref()),
        ) {
            (Some(guild_id), Some(channel)) => write!(f, "{guild_id} `{channel}`"),
            (Some(guild_id), None) => write!(f, "{guild_id}"),
            (None, _) => f.write_str("DM"),
        }
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

pub trait MessageLinkExt {
    /// Sets the guild ID for the link.
    ///
    /// This is useful when the message might not have had the guild ID set.
    fn guild_id(self, guild_id: impl Into<Option<GuildId>>) -> Self;

    /// Converts the link to a key. This will only print the tail of the link.
    fn key(self) -> MessageKey;
}

impl MessageLinkExt for MessageLink {
    fn guild_id(mut self, guild_id: impl Into<Option<GuildId>>) -> Self {
        self.guild_id = guild_id.into();
        self
    }

    fn key(self) -> MessageKey {
        self.into()
    }
}

/// Formattable message key.
///
/// This is the tail section of the link.
#[derive(Clone, Copy, Eq, PartialEq)]
#[must_use]
pub struct MessageKey {
    link: MessageLink,
}

impl From<MessageLink> for MessageKey {
    fn from(value: MessageLink) -> Self {
        Self { link: value }
    }
}

impl From<MessageKey> for MessageLink {
    fn from(value: MessageKey) -> Self {
        value.link
    }
}

impl Display for MessageKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self.link.guild_id {
            Some(guild_id) => write!(f, "{guild_id}/")?,
            None => f.write_str("@me/")?,
        }

        write!(f, "{}/{}", self.link.channel_id, self.link.message_id)
    }
}

/// Implements [`Display`] to format the full command.
#[must_use]
pub struct DisplayCommand<'a> {
    data: &'a CommandData,
    options: &'a [ResolvedOption<'a>],
}

impl<'a> DisplayCommand<'a> {
    pub fn new(data: &'a CommandData, options: &'a [ResolvedOption<'a>]) -> Self {
        Self { data, options }
    }
}

impl Display for DisplayCommand<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if self.data.kind == CommandType::ChatInput {
            f.write_str("/")?;
            f.write_str(&self.data.name)?;
            let mut options = &self.data.options;
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

            fmt_resolved_options(self.options, f)
        } else {
            f.write_str(&self.data.name)?;

            if let Some(target) = self.data.target() {
                f.write_str(": ")?;
                fmt_resolved_target(&target, f)
            } else {
                Ok(())
            }
        }
    }
}

fn fmt_resolved_options(options: &[ResolvedOption<'_>], f: &mut Formatter<'_>) -> Result {
    for o in options {
        f.write_str(" ")?;
        f.write_str(o.name)?;
        f.write_str(": ")?;
        fmt_resolved_option(&o.value, f)?;
    }

    Ok(())
}

fn fmt_resolved_option(value: &ResolvedValue<'_>, f: &mut Formatter<'_>) -> Result {
    match value {
        ResolvedValue::Boolean(v) => v.fmt(f),
        ResolvedValue::Integer(v) => v.fmt(f),
        ResolvedValue::Number(v) => v.fmt(f),
        ResolvedValue::String(v) => write!(f, "\"{v}\""),
        ResolvedValue::Attachment(v) => f.write_str(&v.filename),
        ResolvedValue::Channel(v) => match &v.base().name {
            Some(name) => f.write_str(name),
            None => v.id().fmt(f),
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
