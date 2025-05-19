use std::hint;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use anyhow::Context as _;
use azur_lane::ship::ShipData;
use serenity::builder::CreateEmbedAuthor;
use serenity::small_fixed_array::FixedString;
use utils::join;

use super::GameData;

fn default_early_load() -> bool {
    true
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    data_path: Arc<Path>,
    #[serde(default = "default_early_load")]
    pub early_load: bool,

    #[serde(default)]
    pub wiki_urls: WikiUrls,

    /// Stores the lazy-loaded game data.
    ///
    /// If this holds [`None`], the data could not be loaded. This state is
    /// mapped to an error for callers outside this module.
    #[serde(skip)]
    game_data: OnceLock<Option<GameData>>,
}

impl Config {
    /// Loads the config. Calling this on the same value will always return the
    /// same result.
    ///
    /// Only one thread will load the config. Concurrent callers will wait for
    /// it to finish loading. After it has been loaded, future calls will no
    /// longer block and reuse the results.
    pub fn load(&self) -> anyhow::Result<LoadedConfig<'_>> {
        LoadedConfig::new(self)
    }

    /// Whether the game data has been loaded before.
    pub fn loaded(&self) -> bool {
        matches!(self.game_data.get(), Some(Some(_)))
    }

    /// Whether the game data still needs to be loaded.
    ///
    /// This returns `true` when either the game data hasn't been loaded yet or
    /// a previous attempt failed.
    pub(super) fn needs_load(&self) -> bool {
        self.game_data.get().is_none()
    }

    /// Gets or loads the game data.
    ///
    /// Only one thread will load the game data, other threads will wait for it
    /// to be finish loading. Future calls will return the cached data.
    pub fn game_data(&self) -> anyhow::Result<&GameData> {
        self.game_data
            .get_or_init(|| self.load_game_data())
            .as_ref()
            .context("failed to load azur lane data")
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
                const VAL: &str = join!($($s),*);
                let f = FixedString::from_static_trunc(VAL);
                assert_eq!(VAL.len(), f.len() as usize, "wiki url default too long");
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
    pub fn ship<'s>(&self, base_ship: &'s ShipData) -> CreateEmbedAuthor<'s> {
        let mut wiki_url = (*self.ship_base).to_owned();
        urlencoding::Encoded::new(base_ship.name.as_str()).append_to(&mut wiki_url);
        CreateEmbedAuthor::new(&base_ship.name).url(wiki_url)
    }
}

/// Represents a config with loaded game data.
//
// Note: this type has a safety invariant for the `raw` field: Its `game_data` field must hold
// `Some(GameData)` already. If that's not the case, UB may follow the use of this value.
// Using `LoadedConfig::new` or `Config::load` ensures the data is loaded successfully.
#[derive(Debug, Clone, Copy)]
pub struct LoadedConfig<'a> {
    /// The raw configuration.
    pub raw: &'a Config,

    // this field only serves to make it explicit that this type has safety invariants for
    // construction and to avoid constructing it outside of this module.
    _unsafe: (),
}

impl<'a> LoadedConfig<'a> {
    /// Creates a new [`LoadedConfig`] from a raw value, ensuring that the game
    /// data is loaded. If the game data hasn't been loaded yet, attempts to
    /// load it and returns an error on failure.
    ///
    /// If this function has returned an error once, it will always return one.
    fn new(raw: &'a Config) -> anyhow::Result<Self> {
        let _: &'a GameData = raw.game_data()?;

        // SAFETY: just ensured that the game data is loaded
        Ok(unsafe { Self::new_unchecked(raw) })
    }

    /// Creates a new [`Config`] from a raw value, assuming that the game data
    /// is already loaded.
    ///
    /// # Safety
    ///
    /// The raw config must already hold resolved `game_data`. This is ensured
    /// if [`Config::game_data`] has returned [`Ok`] before.
    unsafe fn new_unchecked(raw: &'a Config) -> Self {
        debug_assert!(
            matches!(raw.game_data.get(), Some(Some(_))),
            "must have initialized `game_data` by now"
        );
        Self { raw, _unsafe: () }
    }

    /// Gets a reference to the wiki URLs.
    pub fn wiki_urls(self) -> &'a WikiUrls {
        &self.raw.wiki_urls
    }

    /// Gets a reference to the game data.
    pub fn game_data(self) -> &'a GameData {
        match self.raw.game_data.get() {
            Some(Some(data)) => data,
            // SAFETY: `new` ensures that the game data is already loaded and not `None`
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }
}
