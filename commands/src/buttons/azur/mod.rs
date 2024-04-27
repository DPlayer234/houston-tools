//use crate::internal::prelude::*;
use crate::buttons::*;

pub mod ship;
pub mod augment;
pub mod skill;

macro_rules! error {
    ($type:ident : $message:literal) => {
        #[derive(Debug, Clone)]
        pub struct $type;

        impl std::error::Error for $type {}

        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, $message)
            }
        }
    };
}

error!(ShipParseError: "Unknown ship.");
error!(AugmentParseError: "Unknown augment.");
error!(SkillParseError: "Unknown skill.");

pub(self) fn get_ship_url(base_ship: &azur_lane::ship::ShipData) -> CreateEmbedAuthor {
    let wiki_url = config::WIKI_BASE_URL.to_owned() + &urlencoding::encode(base_ship.name.as_ref());
    CreateEmbedAuthor::new(base_ship.name.as_ref()).url(wiki_url)
}
