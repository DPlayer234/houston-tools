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

    fn buttons(&self, _config: &HBotConfig) -> impl IntoIterator<Item = ButtonAction> {
        [
            buttons::augment::View::action(),
            buttons::equip::View::action(),
            buttons::juustagram_chat::View::action(),
            buttons::lines::View::action(),
            buttons::search_augment::View::action(),
            buttons::search_equip::View::action(),
            buttons::search_juustagram_chat::View::action(),
            buttons::search_ship::View::action(),
            buttons::search_special_secretary::View::action(),
            buttons::shadow_equip::View::action(),
            buttons::ship::View::action(),
            buttons::skill::View::action(),
            buttons::special_secretary::View::action(),
        ]
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
