use std::sync::OnceLock;

use serenity::http::Http;

use crate::config::HBotConfig;
use crate::modules::{Module as _, for_each_module};
use crate::prelude::*;

mod app_emojis;
pub mod cache;

pub use app_emojis::HAppEmojis;
use cache::Cache;

/// A general color that can be used for embeds indicating errors.
pub const ERROR_EMBED_COLOR: Color = Color::new(0xCF_00_25);

/// Actual data type provided to serenity's user data.
pub type HContextData = HBotData;

/// A simple error that can return any error message.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{msg}")]
pub struct HArgError {
    /// The error message
    pub msg: Cow<'static, str>,
}

impl HArgError {
    pub const fn new_const(msg: &'static str) -> Self {
        Self {
            msg: Cow::Borrowed(msg),
        }
    }

    pub fn new(msg: impl Into<Cow<'static, str>>) -> Self {
        Self { msg: msg.into() }
    }
}

/// The global bot data. Only one instance exists per bot.
#[derive(Debug)]
pub struct HBotData {
    /// The bot configuration.
    config: HBotConfig,
    /// The loaded application emojis.
    app_emojis: OnceLock<app_emojis::HAppEmojiStore>,
    /// The Discord cache.
    pub cache: Arc<Cache>,
    /// Database connection.
    database: OnceLock<mongodb::Database>,
}

impl HBotData {
    /// Creates a new instance.
    #[must_use]
    pub fn new(config: HBotConfig) -> Self {
        Self {
            config,
            app_emojis: OnceLock::new(),
            cache: Arc::default(),
            database: OnceLock::new(),
        }
    }

    /// Gets the bot configuration.
    #[must_use]
    pub fn config(&self) -> &HBotConfig {
        &self.config
    }

    /// Gets the loaded app emojis.
    #[must_use]
    pub fn app_emojis(&self) -> HAppEmojis<'_> {
        HAppEmojis(self.app_emojis.get())
    }

    /// Gets the database connection.
    pub fn database(&self) -> Result<&mongodb::Database> {
        self.database.get().context("database is not connected")
    }

    /// Gets the init data needed based on the enabled modules.
    pub fn init(&self) -> Result<HInit> {
        let config = self.config();
        let mut startup = HInit::default();
        for_each_module!(config, |m| {
            m.validate(config)?;
            startup.intents |= m.intents(config);
            startup.commands.extend(m.commands(config));
            startup.buttons.extend(m.buttons(config));
        });
        Ok(startup)
    }

    /// Performs startup tasks, like connecting to a database and other needed
    /// services. Tasks are run in sequence by default.
    pub async fn startup(self: Arc<Self>) -> Result {
        Arc::clone(&self).connect_database().await?;

        for_each_module!(self.config(), |m| {
            m.startup(Arc::clone(&self)).await?;
        });

        Ok(())
    }

    /// Called in ready to perform finalization with Discord state.
    pub async fn ready(&self, http: &Http) -> Result {
        self.load_app_emojis(http).await
    }

    async fn load_app_emojis(&self, http: &Http) -> Result {
        if self.app_emojis.get().is_none()
            && self
                .app_emojis
                .set(app_emojis::HAppEmojiStore::load_and_update(self.config(), http).await?)
                .is_ok()
        {
            log::info!("Loaded App Emojis.");
        }

        Ok(())
    }

    async fn connect_database(self: Arc<Self>) -> Result {
        if let Some(uri) = &self.config().mongodb_uri {
            let client = mongodb::Client::with_uri_str(uri)
                .await
                .context("failed to connect to database cluster")?;

            let db = client
                .default_database()
                .context("no default database specified")?;

            self.database
                .set(db.clone())
                .expect("can only connect to database once");

            for_each_module!(self.config(), |m| {
                m.db_init(Arc::clone(&self), db.clone()).await?;
            });

            log::info!("Connected to MongoDB.");
        }

        Ok(())
    }
}

/// Data needed for bot startup.
#[derive(Debug)]
pub struct HInit {
    /// Intents used by this app.
    pub intents: GatewayIntents,
    /// Commands to register.
    pub commands: Vec<houston_cmd::model::Command>,
    /// Buttons to register.
    pub buttons: Vec<crate::buttons::ButtonAction>,
}

impl Default for HInit {
    fn default() -> Self {
        Self {
            // default isn't empty but non_privileged
            intents: GatewayIntents::empty(),
            commands: Vec::new(),
            buttons: Vec::new(),
        }
    }
}

pub struct Ephemeral;

pub trait IntoEphemeral {
    fn into_ephemeral(self) -> bool;
}

impl IntoEphemeral for Ephemeral {
    fn into_ephemeral(self) -> bool {
        true
    }
}

impl IntoEphemeral for bool {
    fn into_ephemeral(self) -> bool {
        self
    }
}

impl IntoEphemeral for Option<bool> {
    fn into_ephemeral(self) -> bool {
        self.unwrap_or(true)
    }
}
