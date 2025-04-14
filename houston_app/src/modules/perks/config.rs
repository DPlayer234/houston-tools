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

fn default_check_interval() -> TimeDelta {
    // 2 minutes is about the minimum safe interval for constant role updates
    // we go a little higher since we use this interval for other stuff too
    const { TimeDelta::minutes(3) }
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default = "default_cash_name")]
    pub cash_name: String,
    #[serde(with = "serde_time_delta", default = "default_check_interval")]
    pub check_interval: TimeDelta,
    pub rainbow: Option<RainbowConfig>,
    pub pushpin: Option<PushpinConfig>,
    pub role_edit: Option<RoleEditConfig>,
    pub collectible: Option<CollectibleConfig>,
    pub birthday: Option<BirthdayConfig>,

    #[serde(skip)]
    pub last_check: RwLock<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct EffectPrice {
    pub cost: u32,
    #[serde(with = "serde_time_delta")]
    pub duration: TimeDelta,
}

fn default_item_amount() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct ItemPrice {
    pub cost: u32,
    #[serde(default = "default_item_amount")]
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
    pub channel: GenericChannelId,
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

    #[serde(skip)]
    pub last_check: RwLock<NaiveDate>,
}

#[derive(Debug, serde::Deserialize)]
pub struct BirthdayGuildConfig {
    pub role: Option<RoleId>,
    pub notice: Option<BirthdayNotice>,
    #[serde(with = "check_gifts", default)]
    pub gifts: Vec<(Item, i64)>,
}

#[derive(Debug, serde::Deserialize)]
pub struct BirthdayNotice {
    pub channel: GenericChannelId,
    pub text: String,
}

mod check_gifts {
    use serde::de::{Deserialize as _, Deserializer, Error as _};

    use super::Item;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<(Item, i64)>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = <Vec<(Item, i64)>>::deserialize(deserializer)?;

        for &(_, amount) in &v {
            u32::try_from(amount).map_err(|_| D::Error::custom("birthday gift must fit in u32"))?;
        }

        Ok(v)
    }
}
