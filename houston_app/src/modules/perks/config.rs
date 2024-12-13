use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use indexmap::IndexMap;
use tokio::sync::RwLock;

use super::Item;
use crate::helper::time::serde_time_delta;
use crate::prelude::*;

fn default_cash_name() -> String {
    "$".to_owned()
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default = "default_cash_name")]
    pub cash_name: String,
    pub rainbow: Option<RainbowConfig>,
    pub pushpin: Option<PushpinConfig>,
    pub role_edit: Option<RoleEditConfig>,
    pub collectible: Option<CollectibleConfig>,
    pub birthday: Option<BirthdayConfig>,

    #[serde(skip, default)]
    pub last_check: RwLock<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct EffectPrice {
    pub cost: u32,
    #[serde(with = "serde_time_delta")]
    pub duration: TimeDelta,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct ItemPrice {
    pub cost: u32,
    #[serde(default = "one")]
    pub amount: u32,
}

fn default_rainbow_name() -> String {
    "Rainbow Role".to_owned()
}

fn default_rainbow_description() -> String {
    "A role with regularly changing color.".to_owned()
}

#[derive(Debug, serde::Deserialize)]
pub struct RainbowConfig {
    #[serde(default = "default_rainbow_name")]
    pub name: String,
    #[serde(default = "default_rainbow_description")]
    pub description: String,
    #[serde(flatten)]
    pub price: EffectPrice,
    #[serde(flatten)]
    pub guilds: IndexMap<GuildId, RainbowRoleEntry>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RainbowRoleEntry {
    pub role: RoleId,
}

fn default_pushpin_name() -> String {
    "Pushpin".to_owned()
}

fn default_pushpin_description() -> String {
    "Let's you pin or unpin any message.".to_owned()
}

#[derive(Debug, serde::Deserialize)]
pub struct PushpinConfig {
    #[serde(default = "default_pushpin_name")]
    pub name: String,
    #[serde(default = "default_pushpin_description")]
    pub description: String,
    #[serde(flatten)]
    pub price: ItemPrice,
}

fn default_role_edit_name() -> String {
    "Role Edit".to_owned()
}

fn default_role_edit_description() -> String {
    "Let's you change the name/color of your role.".to_owned()
}

#[derive(Debug, serde::Deserialize)]
pub struct RoleEditConfig {
    #[serde(default = "default_role_edit_name")]
    pub name: String,
    #[serde(default = "default_role_edit_description")]
    pub description: String,
    #[serde(flatten)]
    pub price: ItemPrice,
}

#[derive(Debug, serde::Deserialize)]
pub struct CollectibleConfig {
    pub name: String,
    pub description: String,
    #[serde(flatten)]
    pub price: ItemPrice,
    #[serde(flatten)]
    pub guilds: HashMap<GuildId, CollectibleGuildEntry>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CollectibleGuildEntry {
    pub notice: Option<CollectibleNotice>,
    pub prize_roles: Vec<(u32, RoleId)>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CollectibleNotice {
    pub channel: ChannelId,
    pub text: String,
}

fn default_birthday_duration() -> TimeDelta {
    const { TimeDelta::hours(24) }
}

#[derive(Debug, serde::Deserialize)]
pub struct BirthdayConfig {
    #[serde(with = "serde_time_delta", default = "default_birthday_duration")]
    pub duration: TimeDelta,
    pub regions: Vec<BirthdayRegionConfig>,
    #[serde(default, flatten)]
    pub guilds: IndexMap<GuildId, BirthdayGuildConfig>,
}

#[derive(Debug, serde::Deserialize)]
pub struct BirthdayRegionConfig {
    pub name: String,
    #[serde(with = "serde_time_delta", default)]
    pub time_offset: TimeDelta,

    #[serde(skip, default)]
    pub last_check: RwLock<NaiveDate>,
}

#[derive(Debug, serde::Deserialize)]
pub struct BirthdayGuildConfig {
    pub role: Option<RoleId>,
    pub notice: Option<BirthdayNotice>,
    #[serde(default)]
    pub gifts: Vec<(Item, u32)>,
}

#[derive(Debug, serde::Deserialize)]
pub struct BirthdayNotice {
    pub channel: ChannelId,
    pub text: String,
}
