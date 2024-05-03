use std::sync::Arc;

pub mod buttons;
pub mod slashies;
pub mod config;
mod data;
mod prelude;

/// The error type used for the poise context.
pub type HError = anyhow::Error;
/// The full poise context type.
pub type HContext<'a> = poise::Context<'a, Arc<HBotData>, HError>;
/// The poise command result type.
pub type HResult = Result<(), HError>;

pub use data::*;
