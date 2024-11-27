use crate::buttons::prelude::*;

pub mod augment;
pub mod equip;
pub mod lines;
pub mod search_augment;
pub mod search_equip;
pub mod search_ship;
pub mod shadow_equip;
pub mod ship;
pub mod skill;

#[derive(Debug, thiserror::Error)]
enum AzurParseError {
    #[error("unknown ship")] Ship,
    #[error("unknown equip")] Equip,
    #[error("unknown augment")] Augment,
}

/// Gets the URL to a ship on the wiki.
fn get_ship_wiki_url(base_ship: &azur_lane::ship::ShipData) -> CreateEmbedAuthor<'_> {
    let mut wiki_url = config::azur_lane::WIKI_BASE_URL.to_owned();
    urlencoding::Encoded::new(&base_ship.name).append_to(&mut wiki_url);

    CreateEmbedAuthor::new(&base_ship.name).url(wiki_url)
}

fn get_ship_preview_name<'a>(ctx: &ButtonContext<'a>) -> Option<&'a str> {
    ctx.interaction.message
        .embeds.first()
        .and_then(get_thumbnail_filename)
}

fn get_thumbnail_filename(embed: &Embed) -> Option<&str> {
    let thumb = embed.thumbnail.as_ref()?;
    let (_, name) = thumb.url.rsplit_once('/')?;
    Some(name.split_once('.').map_or(name, |a| a.0))
}

use crate::helper::discord::{
    get_pagination_buttons,
    create_string_select_menu_row,
};
