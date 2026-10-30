use std::convert::Infallible;

use arrayvec::ArrayVec;
use serenity::model::application::{
    Checkbox, CheckboxGroup, CommandDataResolved, FileUpload, InputText, RadioGroup, SelectMenu,
};
use serenity::model::channel::{Attachment, GenericInteractionChannel};
use serenity::model::guild::{PartialMember, Role};
use serenity::model::user::User;

use super::{FromField, ModalResult, marker};

fn parse_map_exact<'a, I, T, const N: usize>(
    values: &'a [I],
    f: impl Fn(&'a I) -> Option<T>,
) -> ModalResult<[T; N], Infallible> {
    if let Ok(values) = <&[_; N]>::try_from(values) {
        values
            .iter()
            .map(f)
            .collect::<Option<ArrayVec<_, N>>>()
            .map(|r| r.into_inner().ok().expect("should be full"))
            .into()
    } else {
        ModalResult::Invalid
    }
}

fn parse_map_vec<I, T>(
    values: &[I],
    f: impl Fn(&I) -> Option<T>,
) -> ModalResult<Vec<T>, Infallible> {
    Option::into(values.iter().map(f).collect())
}

impl<'a> FromField<'a, marker::InputText> for &'a str {
    type Err = Infallible;

    fn from_field(
        field: &'a InputText,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        ModalResult::Ok(&field.value)
    }
}

impl<'a, const N: usize> FromField<'a, marker::SelectMenu> for [&'a str; N] {
    type Err = Infallible;

    fn from_field(
        field: &'a SelectMenu,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        parse_map_exact(&field.values, |i| Some(i.as_str()))
    }
}

impl<'a> FromField<'a, marker::SelectMenu> for Vec<&'a str> {
    type Err = Infallible;

    fn from_field(
        field: &'a SelectMenu,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        ModalResult::Ok(field.values.iter().map(String::as_str).collect())
    }
}

macro_rules! impl_from_select_menu_resolved {
    ($Ty:ty, $data:ident, $select:expr) => {
        impl<'a, const N: usize> FromField<'a, marker::SelectMenu> for [$Ty; N] {
            type Err = Infallible;

            fn from_field(
                field: &'a SelectMenu,
                $data: &'a CommandDataResolved,
            ) -> ModalResult<Self, Self::Err> {
                parse_map_exact(&field.values, |s| s.parse().ok().and_then($select))
            }
        }

        impl<'a> FromField<'a, marker::SelectMenu> for Vec<$Ty> {
            type Err = Infallible;

            fn from_field(
                field: &'a SelectMenu,
                $data: &'a CommandDataResolved,
            ) -> ModalResult<Self, Self::Err> {
                parse_map_vec(&field.values, |s| s.parse().ok().and_then($select))
            }
        }
    };
}

impl_from_select_menu_resolved!(&'a User, data, |id| data.users.get(&id));
impl_from_select_menu_resolved!(User, data, |id| data.users.get(&id).cloned());
impl_from_select_menu_resolved!(&'a PartialMember, data, |id| data.members.get(&id));
impl_from_select_menu_resolved!(PartialMember, data, |id| data.members.get(&id).cloned());
impl_from_select_menu_resolved!(&'a Role, data, |id| data.roles.get(&id));
impl_from_select_menu_resolved!(Role, data, |id| data.roles.get(&id).cloned());
impl_from_select_menu_resolved!(&'a GenericInteractionChannel, data, |id| data
    .channels
    .get(&id));
impl_from_select_menu_resolved!(GenericInteractionChannel, data, |id| data
    .channels
    .get(&id)
    .cloned());

impl_from_select_menu_resolved!((&'a User, &'a PartialMember), data, |id| data
    .users
    .get(&id)
    .zip(data.members.get(&id)));
impl_from_select_menu_resolved!((User, PartialMember), data, |id| data
    .users
    .get(&id)
    .zip(data.members.get(&id))
    .map(|(u, m)| (u.clone(), m.clone())));

impl<'a, const N: usize> FromField<'a, marker::FileUpload> for [&'a Attachment; N] {
    type Err = Infallible;

    fn from_field(
        field: &'a FileUpload,
        data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        parse_map_exact(&field.values, |i| data.attachments.get(i))
    }
}

impl<'a, const N: usize> FromField<'a, marker::FileUpload> for [Attachment; N] {
    type Err = Infallible;

    fn from_field(
        field: &'a FileUpload,
        data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        parse_map_exact(&field.values, |i| data.attachments.get(i).cloned())
    }
}

impl<'a> FromField<'a, marker::FileUpload> for Vec<&'a Attachment> {
    type Err = Infallible;

    fn from_field(
        field: &'a FileUpload,
        data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        parse_map_vec(&field.values, |i| data.attachments.get(i))
    }
}

impl<'a> FromField<'a, marker::FileUpload> for Vec<Attachment> {
    type Err = Infallible;

    fn from_field(
        field: &'a FileUpload,
        data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        parse_map_vec(&field.values, |i| data.attachments.get(i).cloned())
    }
}

impl<'a> FromField<'a, marker::RadioGroup> for &'a str {
    type Err = Infallible;

    fn from_field(
        field: &'a RadioGroup,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        field.value.as_deref().into()
    }
}

impl<'a, const N: usize> FromField<'a, marker::CheckboxGroup> for [&'a str; N] {
    type Err = Infallible;

    fn from_field(
        field: &'a CheckboxGroup,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        parse_map_exact(&field.values, |i| Some(i.as_str()))
    }
}

impl<'a> FromField<'a, marker::CheckboxGroup> for Vec<&'a str> {
    type Err = Infallible;

    fn from_field(
        field: &'a CheckboxGroup,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        ModalResult::Ok(field.values.iter().map(String::as_str).collect())
    }
}

impl<'a> FromField<'a, marker::Checkbox> for bool {
    type Err = Infallible;

    fn from_field(
        field: &'a Checkbox,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        ModalResult::Ok(field.value)
    }
}
