use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use anyhow::Context as _;
use azur_lane::ship::ShipData;
use serenity::builder::CreateEmbedAuthor;
use utils::join;

use super::GameData;

fn default_true() -> bool {
    true
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    data_path: Arc<Path>,
    #[serde(default = "default_true")]
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
    pub ship_base: Cow<'static, str>,
    pub ship_list: Cow<'static, str>,
    pub equipment_list: Cow<'static, str>,
    pub dd_gun_list: Cow<'static, str>,
    pub cl_gun_list: Cow<'static, str>,
    pub ca_gun_list: Cow<'static, str>,
    pub cb_gun_list: Cow<'static, str>,
    pub bb_gun_list: Cow<'static, str>,
    pub surface_torpedo_list: Cow<'static, str>,
    pub sub_torpedo_list: Cow<'static, str>,
    pub aa_gun_list: Cow<'static, str>,
    pub fuze_aa_gun_list: Cow<'static, str>,
    pub auxiliary_list: Cow<'static, str>,
    pub cargo_list: Cow<'static, str>,
    pub anti_sub_list: Cow<'static, str>,
    pub fighter_list: Cow<'static, str>,
    pub dive_bomber_list: Cow<'static, str>,
    pub torpedo_bomber_list: Cow<'static, str>,
    pub seaplane_list: Cow<'static, str>,
    pub augment_list: Cow<'static, str>,
}

impl Default for WikiUrls {
    fn default() -> Self {
        const BASE: &str = "https://azurlane.koumakan.jp/wiki/";
        Self {
            ship_base: BASE.into(),
            ship_list: join!(BASE, "List_of_Ships").into(),
            equipment_list: join!(BASE, "Equipment_List").into(),
            dd_gun_list: join!(BASE, "List_of_Destroyer_Guns").into(),
            cl_gun_list: join!(BASE, "List_of_Light_Cruiser_Guns").into(),
            ca_gun_list: join!(BASE, "List_of_Heavy_Cruiser_Guns").into(),
            cb_gun_list: join!(BASE, "List_of_Large_Cruiser_Guns").into(),
            bb_gun_list: join!(BASE, "List_of_Battleship_Guns").into(),
            surface_torpedo_list: join!(BASE, "List_of_Torpedoes").into(),
            sub_torpedo_list: join!(BASE, "List_of_Submarine_Torpedoes").into(),
            aa_gun_list: join!(BASE, "List_of_AA_Guns").into(),
            fuze_aa_gun_list: join!(BASE, "List_of_AA_Time_Fuze_Guns").into(),
            auxiliary_list: join!(BASE, "List_of_Auxiliary_Equipment").into(),
            cargo_list: join!(BASE, "List_of_Cargo").into(),
            anti_sub_list: join!(BASE, "List_of_ASW_Equipment").into(),
            fighter_list: join!(BASE, "List_of_Fighters").into(),
            dive_bomber_list: join!(BASE, "List_of_Dive_Bombers").into(),
            torpedo_bomber_list: join!(BASE, "List_of_Torpedo_Bombers").into(),
            seaplane_list: join!(BASE, "List_of_Seaplanes").into(),
            augment_list: join!(BASE, "List_of_Augment_Modules").into(),
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
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}
