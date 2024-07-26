use std::sync::Arc;

use dashmap::DashMap;
use once_cell::sync::{Lazy, OnceCell};
use poise::reply::CreateReply;
use serenity::all::{Color, Http, UserId};

mod app_emojis;
mod azur;

use crate::config::HBotConfig;

/// A general color that can be used for various embeds.
pub const DEFAULT_EMBED_COLOR: Color = Color::new(0xDD_A0_DD);

/// A general color that can be used for embeds indicating errors.
pub const ERROR_EMBED_COLOR: Color = Color::new(0xCF_00_25);

/// The error type used for the poise context.
pub type HError = anyhow::Error;
/// The full poise context type.
pub type HContext<'a> = poise::Context<'a, Arc<HBotData>, HError>;
/// The poise command result type.
pub type HResult = Result<(), HError>;

pub use azur::HAzurLane;
pub use app_emojis::HAppEmojis;

/// The global bot data. Only one instance exists per bot.
pub struct HBotData {
    /// The bot configuration.
    config: HBotConfig,
    /// The loaded application emojis.
    app_emojis: OnceCell<app_emojis::HAppEmojiStore>,
    /// A concurrent hash map to user data.
    user_data: DashMap<UserId, HUserData>,
    /// Lazily initialized Azur Lane data.
    azur_lane: Lazy<HAzurLane, Box<dyn Send + FnOnce() -> HAzurLane>>,
}

/// User-specific data.
#[derive(Debug, Clone)]
pub struct HUserData {
    pub ephemeral: bool
}

/// A simple error that can return any error message.
#[derive(Debug, Clone)]
pub struct HArgError(
    /// The error message
    pub &'static str
);

impl std::error::Error for HArgError {}

impl std::fmt::Display for HArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::fmt::Debug for HBotData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(HBotData)).finish()
    }
}

impl HBotData {
    /// Creates a new instance.
    #[must_use]
    pub fn new(config: HBotConfig) -> Self {
        let data_path = config.azur_lane_data.clone();
        HBotData {
            config,
            app_emojis: OnceCell::new(),
            user_data: DashMap::new(),
            azur_lane: Lazy::new(match data_path {
                Some(data_path) => Box::new(move || HAzurLane::load_from(data_path)),
                None => Box::new(HAzurLane::default),
            })
        }
    }

    /// Forces initialization of held lazy data.
    pub fn force_init(&self) {
        let _ = self.azur_lane();
    }

    #[must_use]
    pub fn config(&self) -> &HBotConfig {
        &self.config
    }

    #[must_use]
    pub fn app_emojis(&self) -> HAppEmojis {
        HAppEmojis(self.app_emojis.get())
    }

    pub async fn load_app_emojis(&self, ctx: &Http) -> HResult {
        if self.app_emojis.get().is_none() {
            let _ = self.app_emojis.set(app_emojis::HAppEmojiStore::load_and_update(&self.config, ctx).await?);
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
        HUserData {
            ephemeral: true
        }
    }
}

impl HUserData {
    /// Creates a reply matching the user data.
    #[must_use]
    pub fn create_reply(&self) -> CreateReply {
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
    fn create_reply(&self) -> CreateReply;

    /// Always creates an ephemeral reply.
    #[must_use]
    fn create_ephemeral_reply(&self) -> CreateReply;
}

impl HContextExtensions for HContext<'_> {
    fn get_user_data(&self) -> HUserData {
        self.data().get_user_data(self.author().id)
    }

    fn set_user_data(&self, data: HUserData) {
        self.data().set_user_data(self.author().id, data)
    }

    fn create_reply(&self) -> CreateReply {
        self.get_user_data().create_reply()
    }

    fn create_ephemeral_reply(&self) -> CreateReply {
        CreateReply::default().ephemeral(true)
    }
}
