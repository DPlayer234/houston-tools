use std::str::FromStr;

use arrayvec::ArrayVec;
use serenity::model::application::{
    Checkbox, CheckboxGroup, CommandDataResolved, InputText, RadioGroup, SelectMenu,
};

use super::{FromField, ModalResult, marker};

fn parse_exact<T: FromStr, const N: usize>(values: &[String]) -> ModalResult<[T; N], T::Err> {
    if let Ok(values) = <&[_; N]>::try_from(values) {
        values
            .iter()
            .map(|i| i.parse())
            .collect::<Result<ArrayVec<_, N>, T::Err>>()
            .map(|r| r.into_inner().ok().expect("should be full"))
            .into()
    } else {
        ModalResult::Invalid
    }
}

fn parse_vec<T: FromStr>(values: &[String]) -> ModalResult<Vec<T>, T::Err> {
    Result::into(values.iter().map(|s| s.parse()).collect())
}

impl<'a, T: FromStr> FromField<'a, marker::ParseInputText> for T {
    type Err = T::Err;

    fn from_field(
        field: &'a InputText,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        field.value.parse().into()
    }
}

impl<'a, T: FromStr, const N: usize> FromField<'a, marker::ParseSelectMenu> for [T; N] {
    type Err = T::Err;

    fn from_field(
        field: &'a SelectMenu,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        parse_exact(&field.values)
    }
}

impl<'a, T: FromStr> FromField<'a, marker::ParseSelectMenu> for Vec<T> {
    type Err = T::Err;

    fn from_field(
        field: &'a SelectMenu,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        parse_vec(&field.values)
    }
}

impl<'a, T: FromStr> FromField<'a, marker::ParseRadioGroup> for T {
    type Err = T::Err;

    fn from_field(
        field: &'a RadioGroup,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        if let Some(value) = &field.value {
            value.parse().into()
        } else {
            ModalResult::Invalid
        }
    }
}

impl<'a, T: FromStr, const N: usize> FromField<'a, marker::ParseCheckboxGroup> for [T; N] {
    type Err = T::Err;

    fn from_field(
        field: &'a CheckboxGroup,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        parse_exact(&field.values)
    }
}

impl<'a, T: FromStr> FromField<'a, marker::ParseCheckboxGroup> for Vec<T> {
    type Err = T::Err;

    fn from_field(
        field: &'a CheckboxGroup,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        parse_vec(&field.values)
    }
}

impl<'a, T: TryFrom<bool>> FromField<'a, marker::ParseCheckbox> for T {
    type Err = T::Error;

    fn from_field(
        field: &'a Checkbox,
        _data: &'a CommandDataResolved,
    ) -> ModalResult<Self, Self::Err> {
        field.value.try_into().into()
    }
}
