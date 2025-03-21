pub use std::borrow::Cow;

pub use anyhow::Context as _;
pub use houston_cmd::{CreateReply, EditReply};
pub use serenity::builder::*;
pub use serenity::futures::{StreamExt as _, TryStreamExt as _};
pub use serenity::gateway::client::FullEvent;
pub use serenity::model::prelude::*;

pub(crate) use crate::config;
pub use crate::data::*;
pub use crate::helper::bson::ModelCollection as _;

pub type EmbedFieldCreate<'a> = (Cow<'a, str>, Cow<'a, str>, bool);
pub type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

pub fn embed_field_create<'a>(
    name: impl Into<Cow<'a, str>>,
    value: impl Into<Cow<'a, str>>,
    inline: bool,
) -> EmbedFieldCreate<'a> {
    (name.into(), value.into(), inline)
}
