pub use std::borrow::Cow;
pub use std::sync::Arc;

pub use anyhow::Context as _;
pub use houston_cmd::{CreateReply, EditReply};
pub use serenity::builder::*;
pub use serenity::futures::{StreamExt as _, TryStreamExt as _};
pub use serenity::gateway::client::FullEvent;
pub use serenity::model::prelude::*;

pub(crate) use crate::config;
pub use crate::data::*;
pub use crate::helper::bson::ModelCollection as _;

pub type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;
