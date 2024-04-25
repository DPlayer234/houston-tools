use std::sync::Arc;

pub mod slashies;
pub mod buttons;
mod internal;
mod data;

pub type HError = Box<dyn std::error::Error + Send + Sync>;
pub type HContext<'a> = poise::Context<'a, Arc<HBotData>, HError>;
pub type HResult = Result<(), HError>;

pub use data::*;
