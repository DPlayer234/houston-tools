use serenity::model::application::{Label, LabelComponent};

use super::ModalResult;

pub struct Empty;

pub trait Field {
    type Actual;

    fn unwrap_item<'l>(label: &'l Label, key: &str) -> ModalResult<&'l Self::Actual, Empty>;
}

macro_rules! declare {
    ($variant:ident, |$item:ident| $is_empty:expr) => {
        pub enum $variant {}

        impl Field for $variant {
            type Actual = serenity::model::application::$variant;

            fn unwrap_item<'l>(
                label: &'l Label,
                key: &str,
            ) -> ModalResult<&'l Self::Actual, Empty> {
                if let LabelComponent::$variant($item) = &label.component
                    && $item.custom_id == key
                {
                    if $is_empty {
                        ModalResult::Err(Empty)
                    } else {
                        ModalResult::Ok($item)
                    }
                } else {
                    ModalResult::Invalid
                }
            }
        }
    };
}

declare!(InputText, |item| item.value.is_empty());
declare!(SelectMenu, |item| item.values.is_empty());
declare!(FileUpload, |item| item.values.is_empty());
declare!(RadioGroup, |item| item.value.is_none());
declare!(CheckboxGroup, |item| item.values.is_empty());
declare!(Checkbox, |item| false);

pub struct Parse<T: Field>(T);

impl<T: Field> Field for Parse<T> {
    type Actual = T::Actual;

    fn unwrap_item<'l>(label: &'l Label, key: &str) -> ModalResult<&'l Self::Actual, Empty> {
        T::unwrap_item(label, key)
    }
}

pub type ParseInputText = Parse<InputText>;
pub type ParseSelectMenu = Parse<SelectMenu>;
pub type ParseRadioGroup = Parse<RadioGroup>;
pub type ParseCheckboxGroup = Parse<CheckboxGroup>;
pub type ParseCheckbox = Parse<Checkbox>;
