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

#[must_use]
pub fn id_suffix(id: impl Into<u64>) -> impl Display {
    IdSuffix::new(id.into())
}

#[must_use]
pub fn interaction_location(
    guild_id: Option<GuildId>,
    channel: Option<&GenericInteractionChannel>,
) -> impl Display + '_ {
    enum Location<'a> {
        Dm,
        Guild(IdSuffix),
        Channel(IdSuffix, &'a str),
    }

    impl Display for Location<'_> {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            match self {
                Location::Dm => f.write_str("DM"),
                Location::Guild(guild) => write!(f, "{guild}"),
                Location::Channel(guild, channel) => write!(f, "{guild} `{channel}`"),
            }
        }
    }

    let guild_id = guild_id.map(|g| IdSuffix::new(g.into()));
    let channel_name = channel.and_then(|c| c.base().name.as_deref());
    match (guild_id, channel_name) {
        (Some(guild_id), Some(channel_name)) => Location::Channel(guild_id, channel_name),
        (Some(guild_id), None) => Location::Guild(guild_id),
        (None, _) => Location::Dm,
    }
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
    channel_id: GenericChannelId,
    message_id: MessageId,
}

impl MessageLink {
    /// Creates a new link from the components.
    pub fn new(
        guild_id: impl Into<Option<GuildId>>,
        channel_id: GenericChannelId,
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
struct IdSuffix(u32);

impl Display for IdSuffix {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "..{:07}", self.0)
    }
}

impl IdSuffix {
    fn new(id: u64) -> Self {
        Self((id % 10_000_000) as u32)
    }
}
