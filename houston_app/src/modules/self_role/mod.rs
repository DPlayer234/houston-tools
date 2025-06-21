use super::prelude::*;

pub mod config;
mod slashies;

pub use config::Config;

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &HBotConfig) -> bool {
        !config.self_role.is_empty()
    }

    fn commands(&self, _config: &HBotConfig) -> impl IntoIterator<Item = Command> {
        [slashies::self_role()]
    }
}
