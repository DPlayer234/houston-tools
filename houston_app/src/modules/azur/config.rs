use std::borrow::Cow;
use std::fmt;
use std::path::PathBuf;
use std::sync::OnceLock;

use azur_lane::equip::EquipKind;
use azur_lane::ship::ShipData;
use serenity::builder::CreateEmbedAuthor;
use utils::join;

use super::GameData;

#[derive(Debug, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    data_path: PathBuf,

    pub ship_base_url: Cow<'static, str>,
    pub ship_list_url: Cow<'static, str>,
    pub equipment_list_url: Cow<'static, str>,
    pub dd_gun_list_url: Cow<'static, str>,
    pub cl_gun_list_url: Cow<'static, str>,
    pub ca_gun_list_url: Cow<'static, str>,
    pub cb_gun_list_url: Cow<'static, str>,
    pub bb_gun_list_url: Cow<'static, str>,
    pub surface_torpedo_list_url: Cow<'static, str>,
    pub sub_torpedo_list_url: Cow<'static, str>,
    pub aa_gun_list_url: Cow<'static, str>,
    pub fuze_aa_gun_list_url: Cow<'static, str>,
    pub auxiliary_list_url: Cow<'static, str>,
    pub cargo_list_url: Cow<'static, str>,
    pub anti_sub_list_url: Cow<'static, str>,
    pub fighter_list_url: Cow<'static, str>,
    pub dive_bomber_list_url: Cow<'static, str>,
    pub torpedo_bomber_list_url: Cow<'static, str>,
    pub seaplane_list_url: Cow<'static, str>,
    pub augment_list_url: Cow<'static, str>,

    /// Stores the loaded game data.
    #[serde(skip)]
    game_data: OnceLock<GameData>,
}

impl Default for Config {
    fn default() -> Self {
        const WIKI_BASE_URL: &str = "https://azurlane.koumakan.jp/wiki/";
        Self {
            data_path: Default::default(),

            ship_base_url: WIKI_BASE_URL.into(),
            ship_list_url: join!(WIKI_BASE_URL, "List_of_Ships").into(),
            equipment_list_url: join!(WIKI_BASE_URL, "Equipment_List").into(),
            dd_gun_list_url: join!(WIKI_BASE_URL, "List_of_Destroyer_Guns").into(),
            cl_gun_list_url: join!(WIKI_BASE_URL, "List_of_Light_Cruiser_Guns").into(),
            ca_gun_list_url: join!(WIKI_BASE_URL, "List_of_Heavy_Cruiser_Guns").into(),
            cb_gun_list_url: join!(WIKI_BASE_URL, "List_of_Large_Cruiser_Guns").into(),
            bb_gun_list_url: join!(WIKI_BASE_URL, "List_of_Battleship_Guns").into(),
            surface_torpedo_list_url: join!(WIKI_BASE_URL, "List_of_Torpedoes").into(),
            sub_torpedo_list_url: join!(WIKI_BASE_URL, "List_of_Submarine_Torpedoes").into(),
            aa_gun_list_url: join!(WIKI_BASE_URL, "List_of_AA_Guns").into(),
            fuze_aa_gun_list_url: join!(WIKI_BASE_URL, "List_of_AA_Time_Fuze_Guns").into(),
            auxiliary_list_url: join!(WIKI_BASE_URL, "List_of_Auxiliary_Equipment").into(),
            cargo_list_url: join!(WIKI_BASE_URL, "List_of_Cargo").into(),
            anti_sub_list_url: join!(WIKI_BASE_URL, "List_of_ASW_Equipment").into(),
            fighter_list_url: join!(WIKI_BASE_URL, "List_of_Fighters").into(),
            dive_bomber_list_url: join!(WIKI_BASE_URL, "List_of_Dive_Bombers").into(),
            torpedo_bomber_list_url: join!(WIKI_BASE_URL, "List_of_Torpedo_Bombers").into(),
            seaplane_list_url: join!(WIKI_BASE_URL, "List_of_Seaplanes").into(),
            augment_list_url: join!(WIKI_BASE_URL, "List_of_Augment_Modules").into(),

            game_data: OnceLock::new(),
        }
    }
}

impl Config {
    pub fn game_data(&self) -> &GameData {
        self.game_data
            .get_or_init(|| GameData::load_from(self.data_path.clone()))
    }

    pub fn get_ship_wiki_url<'s>(&self, base_ship: &'s ShipData) -> CreateEmbedAuthor<'s> {
        let mut wiki_url = (*self.ship_base_url).to_owned();
        urlencoding::Encoded::new(&base_ship.name).append_to(&mut wiki_url);

        CreateEmbedAuthor::new(&base_ship.name).url(wiki_url)
    }

    /// Converts the equip slot to a masked link to the appropriate wiki page.
    pub fn get_equip_slot_display(&self, kind: EquipKind) -> impl fmt::Display + '_ {
        struct Slot<'a> {
            label: &'a str,
            url: &'a str,
        }

        impl<'a> Slot<'a> {
            fn new(label: &'a str, url: &'a str) -> Self {
                Self { label, url }
            }
        }

        impl fmt::Display for Slot<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Self { label, url } = self;
                write!(f, "[{label}]({url})")
            }
        }

        match kind {
            EquipKind::DestroyerGun => Slot::new("DD Gun", &self.dd_gun_list_url),
            EquipKind::LightCruiserGun => Slot::new("CL Gun", &self.cl_gun_list_url),
            EquipKind::HeavyCruiserGun => Slot::new("CA Gun", &self.ca_gun_list_url),
            EquipKind::LargeCruiserGun => Slot::new("CB Gun", &self.cb_gun_list_url),
            EquipKind::BattleshipGun => Slot::new("BB Gun", &self.bb_gun_list_url),
            EquipKind::SurfaceTorpedo => Slot::new("Torpedo", &self.surface_torpedo_list_url),
            EquipKind::SubmarineTorpedo => Slot::new("Torpedo", &self.sub_torpedo_list_url),
            EquipKind::AntiAirGun => Slot::new("AA Gun", &self.aa_gun_list_url),
            EquipKind::FuzeAntiAirGun => Slot::new("AA Gun (Fuze)", &self.fuze_aa_gun_list_url),
            EquipKind::Fighter => Slot::new("Fighter", &self.fighter_list_url),
            EquipKind::DiveBomber => Slot::new("Dive Bomber", &self.dive_bomber_list_url),
            EquipKind::TorpedoBomber => Slot::new("Torpedo Bomber", &self.torpedo_bomber_list_url),
            EquipKind::SeaPlane => Slot::new("Seaplane", &self.seaplane_list_url),
            EquipKind::AntiSubWeapon => Slot::new("ASW", &self.anti_sub_list_url),
            EquipKind::AntiSubAircraft => Slot::new("ASW Aircraft", &self.anti_sub_list_url),
            EquipKind::Helicopter => Slot::new("Helicopter", &self.auxiliary_list_url),
            EquipKind::Missile => Slot::new("Missile", &self.surface_torpedo_list_url),
            EquipKind::Cargo => Slot::new("Cargo", &self.cargo_list_url),
            EquipKind::Auxiliary => Slot::new("Auxiliary", &self.auxiliary_list_url),
        }
    }
}
