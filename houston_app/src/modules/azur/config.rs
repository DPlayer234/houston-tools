use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::OnceLock;

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
    game_data: OnceLock<GameData>,
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

impl Config {
    pub fn game_data(&self) -> &GameData {
        self.game_data
            .get_or_init(|| GameData::load_from(self.data_path.clone()))
    }
}

impl WikiUrls {
    pub fn ship<'s>(&self, base_ship: &'s ShipData) -> CreateEmbedAuthor<'s> {
        let mut wiki_url = (*self.ship_base).to_owned();
        urlencoding::Encoded::new(&base_ship.name).append_to(&mut wiki_url);
        CreateEmbedAuthor::new(&base_ship.name).url(wiki_url)
    }
}
