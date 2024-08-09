pub use serenity::builder::*;
pub use serenity::model::prelude::*;
pub use poise::reply::CreateReply;

pub(crate) use crate::config;
pub use crate::data::*;

pub type SimpleEmbedFieldCreate = (&'static str, String, bool);
