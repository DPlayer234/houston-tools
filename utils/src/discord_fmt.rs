use std::fmt::{Display, Formatter, Result as FmtResult, Write};
use serenity::all::{ResolvedOption, ResolvedTarget, ResolvedValue, User};

pub enum DisplayResolvedArgs<'a> {
    Options(DisplayResolvedOptions<'a>),
    Target(DisplayResolvedTarget<'a>),
    String(&'a str),
}

pub struct DisplayResolvedOptions<'a>(&'a [ResolvedOption<'a>]);
pub struct DisplayResolvedTarget<'a>(ResolvedTarget<'a>);

struct DisplayResolvedOption<'a>(&'a ResolvedOption<'a>);

/// Gets a unique username for this user.
/// 
/// This will either be the pomelo username or include the discriminator.
pub fn get_unique_username(user: &User) -> String {
	user.discriminator
		.map(|d| format!("{}#{:04}", user.name, d))
		.unwrap_or_else(|| user.name.to_owned())
}

impl DisplayResolvedArgs<'_> {
    pub fn from_options<'a>(options: &'a [ResolvedOption<'a>]) -> DisplayResolvedArgs<'a> {
        DisplayResolvedArgs::Options(DisplayResolvedOptions(options))
    }

    pub fn from_target<'a>(target: ResolvedTarget<'a>) -> DisplayResolvedArgs<'a> {
        DisplayResolvedArgs::Target(DisplayResolvedTarget(target))
    }

    pub fn from_str<'a>(string: &'a str) -> DisplayResolvedArgs<'a> {
        DisplayResolvedArgs::String(string)
    }
}

impl Display for DisplayResolvedArgs<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            DisplayResolvedArgs::Options(o) => o.fmt(f),
            DisplayResolvedArgs::Target(t) => t.fmt(f),
            DisplayResolvedArgs::String(s) => f.write_str(s),
        }
    }
}

impl Display for DisplayResolvedOptions<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        for o in self.0 {
            f.write_str(o.name)?;
            f.write_char(':')?;
            DisplayResolvedOption(o).fmt(f)?;
            f.write_char(' ')?;
        }
        
        Ok(())
    }
}

impl Display for DisplayResolvedOption<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self.0.value {
            ResolvedValue::Boolean(v) => { write!(f, "{}", v) },
            ResolvedValue::Integer(v) => { write!(f, "{}", v) },
            ResolvedValue::Number(v) => { write!(f, "{}", v) },
            ResolvedValue::String(v) => { f.write_char('"')?; f.write_str(&v)?; f.write_char('"') },
            ResolvedValue::Attachment(v) => { f.write_str(&v.filename) },
            ResolvedValue::Channel(v) => { if let Some(ref name) = v.name { f.write_str(name) } else { write!(f, "{}", v.id) } },
            ResolvedValue::Role(v) => { f.write_str(&v.name) },
            ResolvedValue::User(v, _) => { f.write_str(&v.name) },
            _ => { f.write_str("<unknown>") },
        }
    }
}

impl Display for DisplayResolvedTarget<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self.0 {
            ResolvedTarget::User(v, _) => f.write_str(&v.name),
            ResolvedTarget::Message(v) => write!(f, "{}", v.id),
            _ => f.write_str("<unknown>"),
        }
    }
}
