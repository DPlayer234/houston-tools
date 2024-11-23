use super::prelude::*;

mod slashies;

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &HBotConfig) -> bool {
        super::perks::Module.enabled(config) ||
        super::starboard::Module.enabled(config)
    }

    fn commands(&self, _config: &HBotConfig) -> impl IntoIterator<Item = HCommand> {
        [
            slashies::profile_context(),
            slashies::profile(),
        ]
    }
}
