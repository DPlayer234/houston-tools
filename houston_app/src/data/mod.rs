use std::borrow::Cow;
use std::sync::{LazyLock, OnceLock};

use anyhow::Context as _;
use serenity::http::Http;
use serenity::model::Color;
use serenity::model::user::CurrentUser;

use crate::config::HBotConfig;

mod app_emojis;

/// A general color that can be used for embeds indicating errors.
pub const ERROR_EMBED_COLOR: Color = Color::new(0xCF_00_25);

/// The error type used for the poise context.
pub type HError = anyhow::Error;
/// The full poise context type.
pub type HContext<'a> = poise::ApplicationContext<'a, HFrameworkData, HError>;
/// The full poise context type.
pub type HAnyContext<'a> = poise::Context<'a, HFrameworkData, HError>;
/// The poise command result type.
pub type HResult = Result<(), HError>;
/// The poise framework type.
pub type HFramework = poise::framework::Framework<HFrameworkData, HError>;
/// Actual data type provided to serenity's user data.
pub type HFrameworkData = HBotData;

pub type HCommand = poise::Command<HFrameworkData, HError>;

pub use app_emojis::HAppEmojis;
use crate::modules::azur::data::HAzurLane;
use crate::modules::perks::PerkState;

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
        Self {
            msg: msg.into(),
        }
    }
}

/// The global bot data. Only one instance exists per bot.
#[derive(Debug)]
pub struct HBotData {
    /// The bot configuration.
    config: HBotConfig,
    /// The current bot user.
    current_user: OnceLock<CurrentUser>,
    /// The loaded application emojis.
    app_emojis: OnceLock<app_emojis::HAppEmojiStore>,
    /// Lazily initialized Azur Lane data.
    azur_lane: LazyLock<HAzurLane, Box<dyn Send + FnOnce() -> HAzurLane>>,
    /// State of the perk module.
    perk_state: PerkState,
    /// Database connection.
    database: OnceLock<mongodb::Database>,
}

impl HBotData {
    /// Creates a new instance.
    #[must_use]
    pub fn new(config: HBotConfig) -> Self {
        let data_path = config.azur_lane_data.clone();
        Self {
            config,
            current_user: OnceLock::new(),
            app_emojis: OnceLock::new(),
            azur_lane: LazyLock::new(match data_path {
                Some(data_path) => Box::new(move || HAzurLane::load_from(data_path)),
                None => Box::new(HAzurLane::default),
            }),
            perk_state: PerkState::default(),
            database: OnceLock::new(),
        }
    }

    /// Forces initialization of held lazy data.
    pub fn force_init(&self) {
        _ = self.azur_lane();
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

    /// Loads all app emojis.
    ///
    /// This doesn't return them. Use [`Self::app_emojis`].
    pub async fn load_app_emojis(&self, ctx: &Http) -> HResult {
        if self.app_emojis.get().is_none() {
            _ = self.app_emojis.set(app_emojis::HAppEmojiStore::load_and_update(&self.config, ctx).await?);
            log::info!("Loaded App Emojis.");
        }

        Ok(())
    }

    /// Gets the cached current bot user.
    pub fn current_user(&self) -> anyhow::Result<&CurrentUser> {
        self.current_user.get().context("current user not loaded")
    }

    /// Sets the current bot user.
    pub fn set_current_user(&self, user: CurrentUser) {
        _ = self.current_user.set(user);
    }

    /// Gets the Azur Lane game data.
    #[must_use]
    pub fn azur_lane(&self) -> &HAzurLane {
        &self.azur_lane
    }

    /// Gets the transient perk state.
    #[must_use]
    pub fn perk_state(&self) -> &PerkState {
        &self.perk_state
    }

    /// Connects to the database and other needed services.
    pub async fn connect(&self, init: &crate::modules::Info) -> HResult {
        if let Some(uri) = &self.config.mongodb_uri {
            let client = mongodb::Client::with_uri_str(uri).await?;
            let db = client.default_database().context("no default database specified")?;

            for init in &init.db_init {
                init(&db).await?;
            }

            self.database
                .set(db)
                .expect("do not call connect more than once");

            log::info!("Connected to MongoDB.");
        }

        Ok(())
    }

    /// Gets the database connection.
    pub fn database(&self) -> anyhow::Result<&mongodb::Database> {
        self.database.get().context("database is not yet connected")
    }
}

/// Extension trait for the poise context.
pub trait HContextExtensions<'a> {
    async fn defer_as(&self, ephemeral: impl IntoEphemeral) -> HResult;

    #[must_use]
    fn data_ref(&self) -> &'a HBotData;
}

impl<'a> HContextExtensions<'a> for HContext<'a> {
    async fn defer_as(&self, ephemeral: impl IntoEphemeral) -> HResult {
        self.defer_response(ephemeral.into_ephemeral()).await?;
        Ok(())
    }

    fn data_ref(&self) -> &'a HBotData {
        self.serenity_context().data_ref::<HFrameworkData>()
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
