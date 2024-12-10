use crate::prelude::*;

pub fn create_string_select_menu_row<'a>(
    custom_id: impl Into<Cow<'a, str>>,
    options: impl Into<Cow<'a, [CreateSelectMenuOption<'a>]>>,
    placeholder: impl Into<Cow<'a, str>>,
) -> CreateActionRow<'a> {
    let kind = CreateSelectMenuKind::String {
        options: options.into(),
    };

    let select = CreateSelectMenu::new(custom_id, kind).placeholder(placeholder);
    CreateActionRow::SelectMenu(select)
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
