use crate::prelude::*;

fn default_cash_name() -> String {
    "$".to_owned()
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default = "default_cash_name")]
    pub cash_name: String,
    pub rainbow: Option<RainbowConfig>,
    pub collectible: Option<CollectibleConfig>,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct EffectPrice {
    pub cost: u32,
    pub duration: u32,
}

fn one() -> u32 { 1 }

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct ItemPrice {
    pub cost: u32,
    #[serde(default = "one")]
    pub amount: u32,
}

#[derive(Debug, serde::Deserialize)]
pub struct RainbowConfig {
    #[serde(flatten)]
    pub price: EffectPrice,
    pub role: Vec<RainbowRoleEntry>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RainbowRoleEntry {
    pub guild: GuildId,
    pub role: RoleId,
}

#[derive(Debug, serde::Deserialize)]
pub struct CollectibleConfig {
    pub name: String,
    #[serde(flatten)]
    pub price: ItemPrice,
}
