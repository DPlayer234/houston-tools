use serenity::prelude::*;

use crate::prelude::*;

pub mod azur;
pub mod core;
pub mod starboard;

/// Initialization data.
pub struct Info {
    /// Intents used by this app.
    pub intents: GatewayIntents,
    /// Commands to register.
    pub commands: Vec<HCommand>,
}

impl Info {
    pub fn new() -> Self {
        Self {
            intents: GatewayIntents::empty(),
            commands: Vec::new(),
        }
    }

    pub fn load(&mut self, config: &config::HBotConfig) -> HResult {
        core::Module.apply(self, config)?;
        azur::Module.apply(self, config)?;
        starboard::Module.apply(self, config)?;
        Ok(())
    }
}

pub trait Module {
    /// Whether the module is enabled.
    fn enabled(&self, config: &config::HBotConfig) -> bool;

    /// The intents needed.
    fn intents(&self) -> GatewayIntents {
        GatewayIntents::empty()
    }

    /// Commands for this module.
    fn commands(&self) -> impl IntoIterator<Item = HCommand> {
        []
    }

    /// Validates that the config is good.
    fn validate(&self, config: &config::HBotConfig) -> HResult {
        _ = config;
        Ok(())
    }

    /// Applies the settings if enabled.
    fn apply(&self, init: &mut Info, config: &config::HBotConfig) -> HResult {
        if self.enabled(config) {
            self.validate(config)?;
            init.intents |= self.intents();
            init.commands.extend(self.commands());
        }
        Ok(())
    }
}
