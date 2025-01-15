use super::prelude::*;

pub mod buttons;
mod slashies;

#[derive(Debug, Default, Clone, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    all: bool,
    tic_tac_toe: Option<bool>,
}

pub struct Module;

impl super::Module for Module {
    fn enabled(&self, config: &HBotConfig) -> bool {
        let m = &config.minigame;
        m.tic_tac_toe.unwrap_or(m.all)
    }

    fn commands(&self, config: &HBotConfig) -> impl IntoIterator<Item = super::HCommand> {
        use houston_cmd::model::{CommandOptionData, GroupData};

        let m = &config.minigame;
        let mut sub_commands = Vec::new();

        if m.tic_tac_toe.unwrap_or(m.all) {
            sub_commands.push(slashies::tic_tac_toe());
        }

        let mut command = slashies::root();
        command.data.data = CommandOptionData::Group(GroupData {
            sub_commands: sub_commands.into(),
        });

        [command]
    }
}
