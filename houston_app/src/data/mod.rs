use std::fmt;
use std::sync::{LazyLock, OnceLock};

use dashmap::DashMap;
use poise::reply::CreateReply;
use serenity::all::{Color, Http, UserId};

use crate::config::HBotConfig;

mod app_emojis;
mod azur;

/// A general color that can be used for various embeds.
pub const DEFAULT_EMBED_COLOR: Color = Color::new(0xDD_A0_DD);

/// A general color that can be used for embeds indicating errors.
pub const ERROR_EMBED_COLOR: Color = Color::new(0xCF_00_25);

/// The error type used for the poise context.
pub type HError = anyhow::Error;
/// The full poise context type.
pub type HContext<'a> = poise::Context<'a, HFrameworkData, HError>;
/// The poise command result type.
pub type HResult = Result<(), HError>;
/// The poise framework type.
pub type HFramework = poise::framework::Framework<HFrameworkData, HError>;
/// Actual data type provided to serenity's user data.
pub type HFrameworkData = HBotData;

pub use azur::HAzurLane;
pub use app_emojis::HAppEmojis;

/// The global bot data. Only one instance exists per bot.
pub struct HBotData {
    /// The bot configuration.
    config: HBotConfig,
    /// The loaded application emojis.
    app_emojis: OnceLock<app_emojis::HAppEmojiStore>,
    /// A concurrent hash map to user data.
    user_data: DashMap<UserId, HUserData>,
    /// Lazily initialized Azur Lane data.
    azur_lane: LazyLock<HAzurLane, Box<dyn Send + FnOnce() -> HAzurLane>>,
}

/// User-specific data.
#[derive(Debug, Clone)]
pub struct HUserData {
    pub ephemeral: bool
}

/// A simple error that can return any error message.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{0}")]
pub struct HArgError(
    /// The error message
    pub &'static str
);

impl fmt::Debug for HBotData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(stringify!(HBotData)).finish()
    }
}

impl HBotData {
    /// Creates a new instance.
    #[must_use]
    pub fn new(config: HBotConfig) -> Self {
        let data_path = config.azur_lane_data.clone();
        Self {
            config,
            app_emojis: OnceLock::new(),
            user_data: DashMap::new(),
            azur_lane: LazyLock::new(match data_path {
                Some(data_path) => Box::new(move || HAzurLane::load_from(data_path)),
                None => Box::new(HAzurLane::default),
            })
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

    /// Gets a copy of the user data for the specified user.
    #[must_use]
    pub fn get_user_data(&self, user_id: UserId) -> HUserData {
        match self.user_data.get(&user_id) {
            None => HUserData::default(),
            Some(guard) => guard.clone()
        }
    }

    /// Replaces the user data for the specified user.
    pub fn set_user_data(&self, user_id: UserId, data: HUserData) {
        self.user_data.insert(user_id, data);
    }

    /// Gets the Azur Lane game data.
    #[must_use]
    pub fn azur_lane(&self) -> &HAzurLane {
        &self.azur_lane
    }
}

impl Default for HUserData {
    fn default() -> Self {
        Self {
            ephemeral: true
        }
    }
}

impl HUserData {
    /// Creates a reply matching the user data.
    #[must_use]
    pub fn create_reply<'a>(&self) -> CreateReply<'a> {
        CreateReply::default()
            .ephemeral(self.ephemeral)
    }
}

/// Extension trait for the poise context.
pub trait HContextExtensions {
    /// Gets a copy of the user data for the current user.
    #[must_use]
    fn get_user_data(&self) -> HUserData;

    /// Replaces the user data for the current user.
    fn set_user_data(&self, data: HUserData);

    /// Creates a reply matching the user data.
    #[must_use]
    fn create_reply<'new>(&self) -> CreateReply<'new>;

    /// Always creates an ephemeral reply.
    #[must_use]
    fn create_ephemeral_reply<'new>(&self) -> CreateReply<'new>;
}

impl HContextExtensions for HContext<'_> {
    fn get_user_data(&self) -> HUserData {
        self.data().get_user_data(self.author().id)
    }

    fn set_user_data(&self, data: HUserData) {
        self.data().set_user_data(self.author().id, data)
    }

    fn create_reply<'new>(&self) -> CreateReply<'new> {
        self.get_user_data().create_reply()
    }

    fn create_ephemeral_reply<'new>(&self) -> CreateReply<'new> {
        CreateReply::default().ephemeral(true)
    }
}
