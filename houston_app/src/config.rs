#![allow(dead_code)]
use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HConfig {
    #[serde(default)]
    pub discord: HDiscordConfig,
    #[serde(default)]
    pub bot: HBotConfig,
    #[serde(default)]
    pub log: HLogConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct HDiscordConfig {
    pub token: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct HBotConfig {
    pub azur_lane_data: Option<PathBuf>,
    pub mongodb_uri: Option<String>,
    #[serde(default)]
    pub starboard: crate::modules::starboard::Config,
    pub perks: Option<crate::modules::perks::Config>,
}

#[derive(Debug, Deserialize, Default)]
pub struct HLogConfig {
    pub color: Option<bool>,
    pub default: Option<log::LevelFilter>,
    #[serde(flatten)]
    pub modules: HashMap<String, log::LevelFilter>,
}

impl HConfig {
    pub fn validate(self) -> anyhow::Result<Self> {
        if !cfg!(feature = "db") && self.bot.mongodb_uri.is_some() {
            anyhow::bail!("mongodb_uri requires compiling the db feature");
        }

        Ok(self)
    }
}

pub mod azur_lane {
    use utils::join;

    /// The base URL to the Azur Lane wiki.
    pub const WIKI_BASE_URL: &str = "https://azurlane.koumakan.jp/wiki/";

    pub const SHIP_LIST_URL: &str = join!(WIKI_BASE_URL, "List_of_Ships");
    pub const EQUIPMENT_LIST_URL: &str = join!(WIKI_BASE_URL, "Equipment_List");

    /// URLs to wiki equip pages.
    pub mod equip {
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
