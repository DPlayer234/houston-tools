use anyhow::Context as _;
use serde::Deserialize;
use serenity::model::Color;
use serenity::secrets::Token;

pub mod emoji;
pub mod setup;

pub use emoji::HEmoji;

#[derive(Debug, Deserialize)]
pub struct HConfig {
    pub discord: HDiscordConfig,
    pub bot: HBotConfig,
    #[serde(default)]
    pub log: HLogConfig,
}

#[derive(Debug, Deserialize)]
pub struct HDiscordConfig {
    pub token: Token,
    pub status: Option<String>,
}

const fn default_embed_color() -> Color {
    Color::new(0xDD_A0_DD)
}

// `azur` and `perks` fields are pretty big, but boxing them is probably not
// worth it since this whole struct already gets moved into an `Arc`. At most,
// when the features are disabled, using boxes would save like 2KiB of RAM, and
// I doubt that would even be noticed.
#[derive(Debug, Deserialize)]
pub struct HBotConfig {
    #[serde(default = "default_embed_color")]
    pub embed_color: Color,
    pub azur: Option<crate::modules::azur::Config>,
    pub mongodb_uri: Option<String>,
    #[serde(default)]
    pub media_react: crate::modules::media_react::Config,
    pub perks: Option<crate::modules::perks::Config>,
    pub rep: Option<crate::modules::rep::Config>,
    #[serde(default)]
    pub starboard: crate::modules::starboard::Config,
}

impl HBotConfig {
    pub fn azur(&self) -> anyhow::Result<crate::modules::azur::LoadedConfig<'_>> {
        self.azur_raw()?.load()
    }

    pub fn azur_raw(&self) -> anyhow::Result<&crate::modules::azur::Config> {
        self.azur.as_ref().context("azur must be enabled")
    }

    pub fn perks(&self) -> anyhow::Result<&crate::modules::perks::Config> {
        self.perks.as_ref().context("perks must be enabled")
    }

    pub fn rep(&self) -> anyhow::Result<&crate::modules::rep::Config> {
        self.rep.as_ref().context("rep must be enabled")
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct HLogConfig {
    #[serde(flatten)]
    pub log4rs: log4rs::config::RawConfig,
    #[serde(default)]
    pub panic: bool,
}
