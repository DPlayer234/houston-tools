use super::prelude::*;

pub mod buttons;
mod slashies;

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, _config: &HBotConfig) -> bool {
        true
    }

    fn commands(&self, _config: &HBotConfig) -> impl IntoIterator<Item = Command> {
        [slashies::minigame()]
    }
}
