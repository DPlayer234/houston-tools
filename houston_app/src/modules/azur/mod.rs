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
            buttons::augment::View::ACTION,
            buttons::equip::View::ACTION,
            buttons::juustagram_chat::View::ACTION,
            buttons::lines::View::ACTION,
            buttons::search_augment::View::ACTION,
            buttons::search_equip::View::ACTION,
            buttons::search_juustagram_chat::View::ACTION,
            buttons::search_ship::View::ACTION,
            buttons::search_special_secretary::View::ACTION,
            buttons::shadow_equip::View::ACTION,
            buttons::ship::View::ACTION,
            buttons::skill::View::ACTION,
            buttons::special_secretary::View::ACTION,
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
