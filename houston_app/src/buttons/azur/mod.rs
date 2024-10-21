use crate::buttons::*;

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
fn get_ship_wiki_url(base_ship: &azur_lane::ship::ShipData) -> CreateEmbedAuthor {
    let mut wiki_url = config::azur_lane::WIKI_BASE_URL.to_owned();
    urlencoding::Encoded::new(&base_ship.name).append_to(&mut wiki_url);

    CreateEmbedAuthor::new(&base_ship.name).url(wiki_url)
}

fn get_pagination_buttons<T: ToCustomData>(
    obj: &mut T,
    page_field: impl utils::fields::FieldMut<T, u16>,
    has_next: bool,
) -> Option<CreateActionRow> {
    let page = *page_field.get(obj);
    (page > 0 || has_next).then(move || CreateActionRow::Buttons(vec![
        if page > 0 {
            obj.new_button(&page_field, page - 1, |_| 1)
        } else {
            CreateButton::new("#no-back").disabled(true)
        }.emoji('◀'),

        if has_next {
            obj.new_button(&page_field, page + 1, |_| 2)
        } else {
            CreateButton::new("#no-forward").disabled(true)
        }.emoji('▶'),
    ]))
}
