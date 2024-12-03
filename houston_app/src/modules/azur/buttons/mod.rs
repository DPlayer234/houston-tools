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
    let embed = ctx.interaction.message.embeds.first()?;
    get_thumbnail_filename(embed)
}

fn get_thumbnail_filename(embed: &Embed) -> Option<&str> {
    let thumb = embed.thumbnail.as_ref()?;
    let (_, name) = thumb.url.rsplit_once('/')?;
    Some(name.split_once('.').map_or(name, |a| a.0))
}

use crate::helper::discord::create_string_select_menu_row;

const PAGE_SIZE: usize = 15;

macro_rules! pagination {
    ($rows:ident => $obj:expr, $options:expr, $iter:expr) => {
        if $options.is_empty() {
            if $obj.page == 0 {
                let embed = CreateEmbed::new()
                    .color(ERROR_EMBED_COLOR)
                    .description("No results for that filter.");

                return Ok(CreateReply::new().embed(embed));
            } else {
                return Err(HArgError::new("This page has no data.").into())
            }
        }

        let mut $rows = Vec::new();

        #[allow(clippy::cast_possible_truncation)]
        let page_count = 1 + $obj.page + $iter.count().div_ceil($crate::modules::azur::buttons::PAGE_SIZE) as u16;
        let pagination = $crate::modules::core::buttons::ToPage::build_row(&mut $obj, ::utils::field_mut!(Self: page))
            .exact_page_count(page_count);

        if let Some(pagination) = pagination.end() {
            $rows.push(pagination);
        }
    };
}

pub(crate) use pagination;
