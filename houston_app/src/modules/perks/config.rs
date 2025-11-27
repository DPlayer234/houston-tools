use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use indexmap::IndexMap;
use serenity::small_fixed_array::{FixedArray, FixedString, ValidLength};
use tokio::sync::Mutex;

use super::Item;
use crate::helper::time::serde_time_delta;
use crate::prelude::*;

macro_rules! fsd {
    ($fn:ident = $str:literal) => {
        fn $fn<LenT: ValidLength>() -> FixedString<LenT> {
            let s = $str;
            let f = <FixedString<LenT>>::from_static_trunc(s);
            assert_eq!(s.len(), f.len().to_usize(), "{} too long", stringify!($fn));
            f
        }
    };
}

fsd!(default_cash_name = "$");

fn default_check_interval() -> TimeDelta {
    // 2 minutes is about the minimum safe interval for constant role updates
    // we go a little higher since we use this interval for other stuff too
    const { TimeDelta::minutes(3) }
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default = "default_cash_name")]
    pub cash_name: FixedString<u8>,
    #[serde(with = "serde_time_delta", default = "default_check_interval")]
    pub check_interval: TimeDelta,
    pub rainbow: Option<RainbowConfig>,
    pub pushpin: Option<PushpinConfig>,
    pub role_edit: Option<RoleEditConfig>,
    pub collectible: Option<CollectibleConfig>,
    pub birthday: Option<BirthdayConfig>,

    #[serde(skip)]
    pub last_check: Mutex<DateTime<Utc>>,
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

fsd!(default_rainbow_name = "Rainbow Role");
fsd!(default_rainbow_description = "A role with regularly changing color.");

#[derive(Debug, serde::Deserialize)]
pub struct RainbowConfig {
    #[serde(default = "default_rainbow_name")]
    pub name: FixedString<u8>,
    #[serde(default = "default_rainbow_description")]
    pub description: FixedString<u16>,
    #[serde(flatten)]
    pub price: EffectPrice,
    #[serde(flatten)]
    pub guilds: IndexMap<GuildId, RainbowRoleEntry>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RainbowRoleEntry {
    pub role: RoleId,
}

fsd!(default_pushpin_name = "Pushpin");
fsd!(default_pushpin_description = "Let's you pin or unpin any message.");

#[derive(Debug, serde::Deserialize)]
pub struct PushpinConfig {
    #[serde(default = "default_pushpin_name")]
    pub name: FixedString<u8>,
    #[serde(default = "default_pushpin_description")]
    pub description: FixedString<u16>,
    #[serde(flatten)]
    pub price: ItemPrice,
}

fsd!(default_role_edit_name = "Role Edit");
fsd!(default_role_edit_description = "Let's you change the name/color of your role.");

#[derive(Debug, serde::Deserialize)]
pub struct RoleEditConfig {
    #[serde(default = "default_role_edit_name")]
    pub name: FixedString<u8>,
    #[serde(default = "default_role_edit_description")]
    pub description: FixedString<u16>,
    #[serde(flatten)]
    pub price: ItemPrice,
}

#[derive(Debug, serde::Deserialize)]
pub struct CollectibleConfig {
    pub name: FixedString<u8>,
    pub description: FixedString<u16>,
    #[serde(flatten)]
    pub price: ItemPrice,
    #[serde(flatten)]
    pub guilds: HashMap<GuildId, CollectibleGuildEntry>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CollectibleGuildEntry {
    pub notice: Option<CollectibleNotice>,
    pub prize_roles: FixedArray<(u32, RoleId)>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CollectibleNotice {
    pub channel: GenericChannelId,
    pub text: FixedString,
}

fn default_birthday_duration() -> TimeDelta {
    const { TimeDelta::hours(24) }
}

#[derive(Debug, serde::Deserialize)]
pub struct BirthdayConfig {
    #[serde(with = "serde_time_delta", default = "default_birthday_duration")]
    pub duration: TimeDelta,
    pub regions: FixedArray<BirthdayRegionConfig>,
    #[serde(default, flatten)]
    pub guilds: IndexMap<GuildId, BirthdayGuildConfig>,
}

#[derive(Debug, serde::Deserialize)]
pub struct BirthdayRegionConfig {
    pub name: FixedString<u8>,
    #[serde(with = "serde_time_delta", default)]
    pub time_offset: TimeDelta,

    #[serde(skip)]
    pub last_check: Mutex<NaiveDate>,
}

#[derive(Debug, serde::Deserialize)]
pub struct BirthdayGuildConfig {
    pub role: Option<RoleId>,
    pub notice: Option<BirthdayNotice>,
    #[serde(with = "check_gifts", default)]
    pub gifts: FixedArray<(Item, i64)>,
}

#[derive(Debug, serde::Deserialize)]
pub struct BirthdayNotice {
    pub channel: GenericChannelId,
    pub text: FixedString,
}

mod check_gifts {
    use serde::de::{Deserialize as _, Deserializer, Error as _};
    use serenity::small_fixed_array::FixedArray;

    use super::Item;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<FixedArray<(Item, i64)>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = <FixedArray<(Item, i64)>>::deserialize(deserializer)?;

        for &(_, amount) in &v {
            u32::try_from(amount).map_err(|_| D::Error::custom("birthday gift must fit in u32"))?;
        }

        Ok(v)
    }
}
