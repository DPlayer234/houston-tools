pub mod slashies;
mod internal;
mod data;

pub type HError = Box<dyn std::error::Error + Send + Sync>;
pub type HContext<'a> = poise::Context<'a, HBotData, HError>;
pub type HResult = Result<(), HError>;

pub use data::*;
