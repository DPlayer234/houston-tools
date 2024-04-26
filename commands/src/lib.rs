use std::sync::Arc;

pub mod slashies;
pub mod buttons;
mod prelude;
mod data;
pub mod config;

pub type HError = anyhow::Error;
pub type HContext<'a> = poise::Context<'a, Arc<HBotData>, HError>;
pub type HResult = Result<(), HError>;

pub use data::*;
