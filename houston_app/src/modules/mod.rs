use std::sync::Arc;

use houston_cmd::model::Command;
use serenity::prelude::*;

use crate::prelude::*;

pub mod azur;
pub mod core;
pub mod media_react;
pub mod minigame;
pub mod perks;
pub mod profile;
pub mod starboard;

mod prelude {
    pub use houston_cmd::model::Command;
    pub use serenity::prelude::*;

    pub use super::Module as _;
    pub use crate::config::HBotConfig;
    pub use crate::prelude::*;
}

mod model_prelude {
    pub use anyhow::Context as _;
    pub use bson::oid::ObjectId;
    pub use bson::serde_helpers::chrono_datetime_as_bson_datetime;
    pub use bson::{Bson, Document, doc};
    pub use bson_model::ModelDocument;
    pub use bson_model::Sort::Asc;
    pub use chrono::{DateTime, Utc};
    pub use mongodb::options::{IndexOptions, ReturnDocument};
    pub use mongodb::{Collection, IndexModel};
    pub use serde::{Deserialize, Serialize};
    pub use serenity::model::id::*;

    pub use crate::helper::bson::{ModelCollection, id_as_i64};
    pub use crate::prelude::*;
}

/// Run an expression against every enabled module.
///
/// Syntax is:
///
/// ```ignore
/// for_each_module!(&config, |m| do_stuff(m));
/// ```
macro_rules! for_each_module {
    (@inner $module:expr, $config:expr, |$var:ident| $body:expr) => {{
        let $var = $module;
        if $crate::modules::Module::enabled(&$var, $config) {
            $body
        }
    }};
    ($config:expr, |$var:ident| $body:expr) => {{
        $crate::modules::for_each_module!(@inner $crate::modules::core::Module, $config, |$var| $body);
        $crate::modules::for_each_module!(@inner $crate::modules::azur::Module, $config, |$var| $body);
        $crate::modules::for_each_module!(@inner $crate::modules::minigame::Module, $config, |$var| $body);
        $crate::modules::for_each_module!(@inner $crate::modules::perks::Module, $config, |$var| $body);
        $crate::modules::for_each_module!(@inner $crate::modules::media_react::Module, $config, |$var| $body);
        $crate::modules::for_each_module!(@inner $crate::modules::profile::Module, $config, |$var| $body);
        $crate::modules::for_each_module!(@inner $crate::modules::starboard::Module, $config, |$var| $body);
    }};
}

pub(crate) use for_each_module;

pub trait Module: Sized {
    /// Whether the module is enabled.
    fn enabled(&self, config: &config::HBotConfig) -> bool;

    /// The intents needed.
    fn intents(&self, config: &config::HBotConfig) -> GatewayIntents {
        _ = config;
        GatewayIntents::empty()
    }

    /// Commands for this module.
    fn commands(&self, config: &config::HBotConfig) -> impl IntoIterator<Item = Command> {
        _ = config;
        []
    }

    /// Validates that the config is good.
    fn validate(&self, config: &config::HBotConfig) -> Result {
        _ = config;
        Ok(())
    }

    /// Runs async startup code for this module.
    async fn startup(self, data: Arc<HBotData>) -> Result {
        _ = data;
        Ok(())
    }

    /// Provides a function to call to initialize the database.
    ///
    /// This will generally create indices on related collection.
    async fn db_init(self, data: Arc<HBotData>, db: mongodb::Database) -> Result {
        _ = data;
        _ = db;
        Ok(())
    }
}
