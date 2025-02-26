use std::sync::Arc;

use super::prelude::*;

pub mod buttons;
pub mod config;
mod data;
mod slashies;

pub use config::{Config, LoadedConfig};
pub use data::GameData;

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &HBotConfig) -> bool {
        config.azur.is_some()
    }

    fn commands(&self, _config: &HBotConfig) -> impl IntoIterator<Item = Command> {
        [slashies::azur()]
    }

    async fn startup(self, data: Arc<HBotData>) -> Result {
        let azur = data.config().azur_raw().unwrap();
        if azur.early_load {
            // load the data on its own thread if requested
            let load = move || _ = data.config().azur();
            tokio::task::spawn_blocking(load);
        }

        Ok(())
    }
}
