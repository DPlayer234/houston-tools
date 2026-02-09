use std::path::Path;
use std::sync::{Arc, OnceLock};

use anyhow::Context as _;
use azur_lane::ship::Ship;
use serenity::small_fixed_array::FixedString;
use utils::join;

use super::GameData;

fn default_early_load() -> bool {
    true
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub data_path: Arc<Path>,
    #[serde(default = "default_early_load")]
    pub early_load: bool,
    #[serde(default)]
    wiki_urls: Arc<WikiUrls>,

    /// Stores the lazy-loaded data.
    ///
    /// If this holds [`None`], the data could not be loaded. This state is
    /// mapped to an error for callers outside this module.
    #[serde(skip)]
    lazy: OnceLock<Option<Box<LazyData>>>,
}

impl Config {
    /// Gets a string describing the load state of the lazy data.
    pub fn load_state(&self) -> &'static str {
        match self.lazy.get() {
            None => "pending",
            Some(None) => "failed",
            Some(Some(_)) => "loaded",
        }
    }

    /// Gets or loads the lazy data.
    ///
    /// Only one thread will load this data, other threads will wait for it
    /// to finish loading. Future calls will return the cached data.
    pub fn lazy(&self) -> anyhow::Result<&LazyData> {
        self.lazy
            .get_or_init(|| self.load_lazy_data())
            .as_deref()
            .context("failed to load azur lane data")
    }

    fn load_lazy_data(&self) -> Option<Box<LazyData>> {
        Some(Box::new(LazyData {
            game_data: self.load_game_data()?,
            wiki_urls: Arc::clone(&self.wiki_urls),
        }))
    }

    fn load_game_data(&self) -> Option<GameData> {
        // this may take a few seconds to load up
        let data_path = Arc::clone(&self.data_path);
        tokio::task::block_in_place(|| GameData::load_from(data_path))
            .inspect(|_| log::info!("Loaded Azur Lane data."))
            .inspect_err(|why| log::error!("Failed to load Azur Lane data: {why:?}"))
            .ok()
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct WikiUrls {
    pub ship_base: FixedString<u16>,
    pub ship_list: FixedString<u16>,
    pub equipment_list: FixedString<u16>,
    pub dd_gun_list: FixedString<u16>,
    pub cl_gun_list: FixedString<u16>,
    pub ca_gun_list: FixedString<u16>,
    pub cb_gun_list: FixedString<u16>,
    pub bb_gun_list: FixedString<u16>,
    pub surface_torpedo_list: FixedString<u16>,
    pub sub_torpedo_list: FixedString<u16>,
    pub aa_gun_list: FixedString<u16>,
    pub fuze_aa_gun_list: FixedString<u16>,
    pub auxiliary_list: FixedString<u16>,
    pub cargo_list: FixedString<u16>,
    pub anti_sub_list: FixedString<u16>,
    pub fighter_list: FixedString<u16>,
    pub dive_bomber_list: FixedString<u16>,
    pub torpedo_bomber_list: FixedString<u16>,
    pub seaplane_list: FixedString<u16>,
    pub augment_list: FixedString<u16>,
}

impl Default for WikiUrls {
    fn default() -> Self {
        macro_rules! fs {
            ($($s:expr),*) => {{
                let val = join!($($s),*);
                let f = FixedString::from_static_trunc(val);
                assert_eq!(val.len(), f.len() as usize, "wiki url default too long");
                f
            }};
        }

        const BASE: &str = "https://azurlane.koumakan.jp/wiki/";
        Self {
            ship_base: fs!(BASE),
            ship_list: fs!(BASE, "List_of_Ships"),
            equipment_list: fs!(BASE, "Equipment_List"),
            dd_gun_list: fs!(BASE, "List_of_Destroyer_Guns"),
            cl_gun_list: fs!(BASE, "List_of_Light_Cruiser_Guns"),
            ca_gun_list: fs!(BASE, "List_of_Heavy_Cruiser_Guns"),
            cb_gun_list: fs!(BASE, "List_of_Large_Cruiser_Guns"),
            bb_gun_list: fs!(BASE, "List_of_Battleship_Guns"),
            surface_torpedo_list: fs!(BASE, "List_of_Torpedoes"),
            sub_torpedo_list: fs!(BASE, "List_of_Submarine_Torpedoes"),
            aa_gun_list: fs!(BASE, "List_of_AA_Guns"),
            fuze_aa_gun_list: fs!(BASE, "List_of_AA_Time_Fuze_Guns"),
            auxiliary_list: fs!(BASE, "List_of_Auxiliary_Equipment"),
            cargo_list: fs!(BASE, "List_of_Cargo"),
            anti_sub_list: fs!(BASE, "List_of_ASW_Equipment"),
            fighter_list: fs!(BASE, "List_of_Fighters"),
            dive_bomber_list: fs!(BASE, "List_of_Dive_Bombers"),
            torpedo_bomber_list: fs!(BASE, "List_of_Torpedo_Bombers"),
            seaplane_list: fs!(BASE, "List_of_Seaplanes"),
            augment_list: fs!(BASE, "List_of_Augment_Modules"),
        }
    }
}

impl WikiUrls {
    pub fn ship(&self, base_ship: &Ship) -> String {
        format!(
            "{}{}",
            self.ship_base,
            urlencoding::Encoded::new(base_ship.base.name.as_str())
        )
    }
}

/// Lazily-loaded data based on the [`Config`].
#[derive(Debug)]
pub struct LazyData {
    // if `Config` were to get more fields that need to be shared here, adjust `Config` to `Arc` a
    // bundle of those fields and clone said `Arc` instead of one per field
    wiki_urls: Arc<WikiUrls>,
    // this is actually all that's lazy-loaded currently
    game_data: GameData,
}

impl LazyData {
    /// Gets a reference to the wiki URLs.
    pub fn wiki_urls(&self) -> &WikiUrls {
        &self.wiki_urls
    }

    /// Gets a reference to the game data.
    pub fn game_data(&self) -> &GameData {
        &self.game_data
    }
}
