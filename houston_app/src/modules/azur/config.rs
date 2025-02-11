use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::Context as _;
use azur_lane::ship::ShipData;
use serenity::builder::CreateEmbedAuthor;
use utils::join;

use super::GameData;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    data_path: PathBuf,

    #[serde(default)]
    pub wiki_urls: WikiUrls,

    /// Stores the loaded game data.
    #[serde(skip)]
    game_data: OnceLock<Option<GameData>>,
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

#[derive(Debug, Clone, Copy)]
pub struct LoadedConfig<'a> {
    pub raw: &'a Config,
    _unsafe: (),
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

impl Config {
    pub fn load(&self) -> anyhow::Result<LoadedConfig<'_>> {
        LoadedConfig::new(self)
    }

    pub fn game_data(&self) -> anyhow::Result<&GameData> {
        self.game_data
            .get_or_init(|| match GameData::load_from(&self.data_path) {
                Ok(data) => Some(data),
                Err(why) => {
                    log::error!("Failed to load Azur Lane data: {why:?}");
                    None
                },
            })
            .as_ref()
            .context("failed to load azur lane data")
    }
}

impl WikiUrls {
    pub fn ship<'s>(&self, base_ship: &'s ShipData) -> CreateEmbedAuthor<'s> {
        let mut wiki_url = (*self.ship_base).to_owned();
        urlencoding::Encoded::new(&base_ship.name).append_to(&mut wiki_url);
        CreateEmbedAuthor::new(&base_ship.name).url(wiki_url)
    }
}

impl<'a> LoadedConfig<'a> {
    /// Creates a new [`Config`] from a raw value, ensuring that the game data
    /// is loaded. If the game data hasn't been loaded yet, attempts to load it
    /// and returns an error on failure.
    ///
    /// If this function has returned an error once, it will always return one.
    fn new(raw: &'a Config) -> anyhow::Result<Self> {
        _ = raw.game_data()?;
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

    pub fn wiki_urls(self) -> &'a WikiUrls {
        &self.raw.wiki_urls
    }

    pub fn game_data(self) -> &'a GameData {
        // SAFETY: `new` ensures that the game data is already loaded and not `None`
        unsafe {
            self.raw
                .game_data
                .get()
                .unwrap_unchecked()
                .as_ref()
                .unwrap_unchecked()
        }
    }
}
