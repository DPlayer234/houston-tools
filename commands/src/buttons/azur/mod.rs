use crate::buttons::*;

pub mod ship;
pub mod augment;
pub mod skill;
pub mod lines;

utils::define_simple_error!(ShipParseError: "Unknown ship.");
utils::define_simple_error!(AugmentParseError: "Unknown augment.");
utils::define_simple_error!(SkillParseError: "Unknown skill.");

pub(self) fn get_ship_url(base_ship: &azur_lane::ship::ShipData) -> CreateEmbedAuthor {
    let wiki_url = config::WIKI_BASE_URL.to_owned() + &urlencoding::encode(&base_ship.name);
    CreateEmbedAuthor::new(&base_ship.name).url(wiki_url)
}
