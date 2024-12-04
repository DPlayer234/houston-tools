#![allow(dead_code, reason = "config might be partly unused")]
use std::path::PathBuf;

use serde::Deserialize;
use serenity::model::Color;
use serenity::secrets::Token;

pub mod azur_lane;
mod token_parse;

#[derive(Debug, Deserialize)]
pub struct HConfig {
    pub discord: HDiscordConfig,
    pub bot: HBotConfig,
    #[serde(default)]
    pub log: log4rs::config::RawConfig,
}

#[derive(Debug, Deserialize)]
pub struct HDiscordConfig {
    #[serde(with = "token_parse")]
    pub token: Token,
    pub status: Option<String>,
}

const fn default_embed_color() -> Color {
    Color::new(0xDD_A0_DD)
}

#[derive(Debug, Deserialize)]
pub struct HBotConfig {
    #[serde(default = "default_embed_color")]
    pub embed_color: Color,
    pub azur_lane_data: Option<PathBuf>,
    pub mongodb_uri: Option<String>,
    #[serde(default)]
    pub media_react: crate::modules::media_react::Config,
    #[serde(default)]
    pub starboard: crate::modules::starboard::Config,
    pub perks: Option<crate::modules::perks::Config>,
}

impl HBotConfig {
    pub fn perks(&self) -> anyhow::Result<&crate::modules::perks::Config> {
        use anyhow::Context as _;
        self.perks.as_ref().context("perks must be enabled")
    }
}
