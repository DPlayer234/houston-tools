use crate::buttons::*;

pub mod augment;
pub mod equip;
pub mod lines;
pub mod search_equip;
pub mod search_ship;
pub mod shadow_equip;
pub mod ship;
pub mod skill;
mod fmt_shared;

utils::define_simple_error!(ShipParseError: "Unknown ship.");
utils::define_simple_error!(EquipParseError: "Unknown equipment.");
utils::define_simple_error!(AugmentParseError: "Unknown augment.");
utils::define_simple_error!(SkillParseError: "Unknown skill.");

/// Gets the URL to a ship on the wiki.
pub(self) fn get_ship_wiki_url(base_ship: &azur_lane::ship::ShipData) -> CreateEmbedAuthor {
    let wiki_url = config::azur_lane::WIKI_BASE_URL.to_owned() + &urlencoding::encode(&base_ship.name);
    CreateEmbedAuthor::new(&base_ship.name).url(wiki_url)
}
