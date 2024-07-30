#![allow(dead_code)]
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HConfig {
    pub discord: HDiscordConfig,
    pub bot: HBotConfig,
    #[serde(default)]
    pub log: HLogConfig,
}

#[derive(Debug, Deserialize)]
pub struct HDiscordConfig {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct HBotConfig {
    pub azur_lane_data: Option<PathBuf>,
    pub trusted_users: HashSet<u64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct HLogConfig {
    pub default: Option<HLogLevel>,
    #[serde(flatten)]
    pub modules: HashMap<String, HLogLevel>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HLogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<HLogLevel> for log::LevelFilter {
    fn from(value: HLogLevel) -> log::LevelFilter {
        use log::LevelFilter;
        match value {
            HLogLevel::Off => LevelFilter::Off,
            HLogLevel::Error => LevelFilter::Error,
            HLogLevel::Warn => LevelFilter::Warn,
            HLogLevel::Info => LevelFilter::Info,
            HLogLevel::Debug => LevelFilter::Debug,
            HLogLevel::Trace => LevelFilter::Trace,
        }
    }
}

pub mod azur_lane {
    /// The base URL to the Azur Lane wiki.
    pub const WIKI_BASE_URL: &str = "https://azurlane.koumakan.jp/wiki/";

    /// URLs to wiki equip pages.
    pub mod equip {
        use utils::join;
        use super::*;

        pub const DD_GUN_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_Destroyer_Guns");
        pub const CL_GUN_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_Light_Cruiser_Guns");
        pub const CA_GUN_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_Heavy_Cruiser_Guns");
        pub const CB_GUN_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_Large_Cruiser_Guns");
        pub const BB_GUN_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_Battleship_Guns");
        pub const SURFACE_TORPEDO_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_Torpedoes");
        pub const SUB_TORPEDO_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_Submarine_Torpedoes");
        pub const AA_GUN_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_AA_Guns");
        pub const FUZE_AA_GUN_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_AA_Time_Fuze_Guns");
        pub const AUXILIARY_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_Auxiliary_Equipment");
        pub const CARGO_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_Cargo");
        pub const ANTI_SUB_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_ASW_Equipment");
        pub const FIGHTER_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_Fighters");
        pub const DIVE_BOMBER_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_Dive_Bombers");
        pub const TORPEDO_BOMBER_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_Torpedo_Bombers");
        pub const SEAPLANE_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_Seaplanes");

        pub const AUGMENT_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_Augment_Modules");
    }
}
