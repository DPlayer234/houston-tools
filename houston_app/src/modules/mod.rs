use serenity::prelude::*;

use crate::prelude::*;

pub mod azur;
pub mod core;
pub mod media_react;
pub mod perks;
pub mod profile;
pub mod starboard;

mod prelude {
    pub use serenity::prelude::*;

    pub use super::Module as _;
    pub(super) use super::HCommand;
    pub use crate::prelude::*;
    pub use crate::config::HBotConfig;
}

type DbInitFn = fn(&mongodb::Database) -> mongodb::BoxFuture<'_, Result>;
type HCommand = houston_cmd::model::Command;

/// Initialization data.
pub struct Info {
    /// Intents used by this app.
    pub intents: GatewayIntents,
    /// Commands to register.
    pub commands: Vec<HCommand>,
    /// DB initializer functions.
    pub db_init: Vec<DbInitFn>,
}

impl Info {
    pub fn new() -> Self {
        Self {
            intents: GatewayIntents::empty(),
            commands: Vec::new(),
            db_init: Vec::new(),
        }
    }

    pub fn load(&mut self, config: &config::HBotConfig) -> Result {
        core::Module.apply(self, config)?;
        azur::Module.apply(self, config)?;
        perks::Module.apply(self, config)?;
        media_react::Module.apply(self, config)?;
        profile::Module.apply(self, config)?;
        starboard::Module.apply(self, config)?;
        Ok(())
    }
}

pub trait Module {
    /// Whether the module is enabled.
    fn enabled(&self, config: &config::HBotConfig) -> bool;

    /// The intents needed.
    fn intents(&self, config: &config::HBotConfig) -> GatewayIntents {
        _ = config;
        GatewayIntents::empty()
    }

    /// Commands for this module.
    fn commands(&self, config: &config::HBotConfig) -> impl IntoIterator<Item = HCommand> {
        _ = config;
        []
    }

    /// Validates that the config is good.
    fn validate(&self, config: &config::HBotConfig) -> Result {
        _ = config;
        Ok(())
    }

    fn db_init(db: &mongodb::Database) -> mongodb::BoxFuture<'_, Result> {
        _ = db;
        Box::pin(const {
            crate::helper::future::Done::new_zst(|| Ok(()))
        })
    }

    /// Applies the settings if enabled.
    fn apply(&self, init: &mut Info, config: &config::HBotConfig) -> Result {
        if self.enabled(config) {
            self.validate(config)?;
            init.intents |= self.intents(config);
            init.commands.extend(self.commands(config));

            if config.mongodb_uri.is_some() {
                init.db_init.push(Self::db_init);
            }
        }
        Ok(())
    }
}
