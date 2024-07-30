use crate::buttons::*;

pub mod augment;
pub mod equip;
pub mod lines;
pub mod search_equip;
pub mod search_ship;
pub mod shadow_equip;
pub mod ship;
pub mod skill;

utils::define_simple_error!(ShipParseError: "unknown ship");
utils::define_simple_error!(EquipParseError: "unknown equipment");
utils::define_simple_error!(AugmentParseError: "unknown augment");

/// Gets the URL to a ship on the wiki.
pub(self) fn get_ship_wiki_url(base_ship: &azur_lane::ship::ShipData) -> CreateEmbedAuthor {
    let wiki_url = config::azur_lane::WIKI_BASE_URL.to_owned() + &urlencoding::encode(&base_ship.name);
    CreateEmbedAuthor::new(&base_ship.name).url(wiki_url)
}
