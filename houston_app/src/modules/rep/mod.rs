use super::prelude::*;

mod config;
pub mod model;
mod slashies;

pub use config::Config;

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &HBotConfig) -> bool {
        config.rep.is_some()
    }

    fn commands(&self, _config: &HBotConfig) -> impl IntoIterator<Item = Command> {
        [slashies::rep(), slashies::rep_context()]
    }

    fn validate(&self, config: &HBotConfig) -> Result {
        let rep = config.rep().unwrap();

        anyhow::ensure!(
            rep.cash_gain == 0 || crate::modules::perks::Module.enabled(config),
            "setting `rep.cash_gain` requires enabling `perks`",
        );

        Ok(())
    }
}
