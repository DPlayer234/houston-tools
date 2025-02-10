use super::prelude::*;

pub mod buttons;
pub mod config;
mod data;
mod slashies;

pub use config::Config;
pub use data::GameData;

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &HBotConfig) -> bool {
        config.azur.is_some()
    }

    fn commands(&self, _config: &HBotConfig) -> impl IntoIterator<Item = HCommand> {
        [slashies::azur()]
    }
}
