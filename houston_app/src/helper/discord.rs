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

pub trait WithPartial {
    type Partial;
}

#[derive(Debug)]
pub enum PartialRef<'a, T: WithPartial> {
    Full(&'a T),
    Partial(&'a T::Partial),
}

impl<T: WithPartial> Copy for PartialRef<'_, T> {}
impl<T: WithPartial> Clone for PartialRef<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl WithPartial for Member {
    type Partial = PartialMember;
}

/// Serializes a Discord ID as an [`u64`].
pub mod id_as_u64 {
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: From<u64>,
    {
        let int = u64::deserialize(deserializer)?;
        if int != u64::MAX {
            Ok(T::from(int))
        } else {
            Err(D::Error::custom("invalid discord id"))
        }
    }

    pub fn serialize<S, T>(val: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Into<u64> + Copy,
    {
        let int: u64 = (*val).into();
        int.serialize(serializer)
    }
}
