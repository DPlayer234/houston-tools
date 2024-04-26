pub use crate::{HContext, HError, HResult};
pub use crate::data::*;
pub use crate::config;
pub use serenity::builder::*;
pub use serenity::model::prelude::*;
pub use serenity::utils::{MessageBuilder, EmbedMessageBuilding};
pub use poise::reply::CreateReply;

pub type SimpleEmbedFieldCreate = (&'static str, String, bool);
