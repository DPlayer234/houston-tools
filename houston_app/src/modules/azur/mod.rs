use super::prelude::*;

pub mod buttons;
pub mod data;
mod slashies;

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &HBotConfig) -> bool {
        config.azur_lane_data.is_some()
    }

    fn commands(&self, _config: &HBotConfig) -> impl IntoIterator<Item = HCommand> {
        [slashies::azur()]
    }
}
