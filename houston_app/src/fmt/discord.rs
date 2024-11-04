//! Provides utilities for formatting Discord data.

use std::borrow::Cow;
use std::fmt::{Display, Formatter, Result};

use serenity::all::{ResolvedOption, ResolvedTarget, ResolvedValue, User};

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
    utils::text::escape_by_char(
        input,
        |c| matches!(c, '*' | '`' | '_' | '>'),
        |c| ['\\', c]
    )
}

/// Implements [`Display`] to format resolved command arguments.
#[must_use]
pub enum DisplayResolvedArgs<'a> {
    /// Uses resolved options from a slash command.
    Options(&'a [ResolvedOption<'a>]),
    /// Uses the resolved target from a context menu command.
    Target(ResolvedTarget<'a>),
    /// Uses the input string from a message command.
    String(&'a str),
}

impl Display for DisplayResolvedArgs<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            DisplayResolvedArgs::Options(o) => fmt_resolved_options(o, f),
            DisplayResolvedArgs::Target(t) => fmt_resolved_target(t, f),
            DisplayResolvedArgs::String(s) => f.write_str(s),
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
        ResolvedValue::Channel(v) => match &v.name { Some(name) => f.write_str(name), None => v.id.fmt(f) },
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
