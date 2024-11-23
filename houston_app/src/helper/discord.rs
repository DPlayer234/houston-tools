use std::borrow::Cow;

use crate::buttons::ToCustomData;
use crate::prelude::*;

pub fn create_string_select_menu_row<'a>(
    custom_id: impl Into<Cow<'a, str>>,
    options: impl Into<Cow<'a, [CreateSelectMenuOption<'a>]>>,
    placeholder: impl Into<Cow<'a, str>>,
) -> CreateActionRow<'a> {
    let kind = CreateSelectMenuKind::String {
        options: options.into(),
    };

    CreateActionRow::SelectMenu(
        CreateSelectMenu::new(custom_id, kind)
            .placeholder(placeholder)
    )
}

pub fn get_pagination_buttons<'a, T: ToCustomData>(
    obj: &mut T,
    page_field: impl utils::fields::FieldMut<T, u16>,
    has_next: bool,
) -> Option<CreateActionRow<'a>> {
    let page = *page_field.get(obj);
    (page > 0 || has_next).then(move || CreateActionRow::buttons(vec![
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
