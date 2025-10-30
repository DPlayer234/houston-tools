use serenity::builder::*;

use super::{IntoComponent, IntoSectionComponent};

macro_rules! impl_into_component {
    ($Ty:ty, $var:ident) => {
        impl<'a> IntoComponent<'a> for $Ty {
            fn into_component(self) -> CreateComponent<'a> {
                CreateComponent::$var(self)
            }
        }
    };
}

macro_rules! impl_into_section_component {
    ($Ty:ty, $var:ident) => {
        impl<'a> IntoSectionComponent<'a> for $Ty {
            fn into_section_component(self) -> CreateSectionComponent<'a> {
                CreateSectionComponent::$var(self)
            }
        }
    };
}

impl_into_component!(CreateActionRow<'a>, ActionRow);
impl_into_component!(CreateSection<'a>, Section);
impl_into_component!(CreateTextDisplay<'a>, TextDisplay);
impl_into_component!(CreateMediaGallery<'a>, MediaGallery);
impl_into_component!(CreateFile<'a>, File);
impl_into_component!(CreateSeparator, Separator);
impl_into_component!(CreateContainer<'a>, Container);

impl_into_section_component!(CreateTextDisplay<'a>, TextDisplay);
