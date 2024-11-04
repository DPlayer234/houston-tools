pub use poise::reply::CreateReply;
pub use serenity::builder::*;
pub use serenity::model::prelude::*;

pub(crate) use crate::config;
pub use crate::data::*;

pub type SimpleEmbedFieldCreate<'a> = (&'a str, String, bool);
