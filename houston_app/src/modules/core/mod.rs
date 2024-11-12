pub mod buttons;
mod slashies;

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, _config: &super::config::HBotConfig) -> bool {
        true
    }

    fn commands(&self) -> impl IntoIterator<Item = super::HCommand> {
        [
            slashies::coin::coin(),
            slashies::dice::dice(),
            slashies::calc::calc(),
            slashies::quote::quote(),
            slashies::timestamp::timestamp(),
            slashies::who::who(),
            slashies::who::who_context(),
            slashies::upload::upload(),
        ]
    }
}