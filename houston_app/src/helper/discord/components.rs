use std::ops::{Deref, DerefMut};

use serenity::small_fixed_array::FixedString;

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

/// A collection of [`CreateComponent`]s.
///
/// This is a thin wrapper around a [`Vec`] and derefs to it.
#[derive(Debug, Clone, Default)]
#[repr(transparent)]
pub struct CreateComponents<'a>(Vec<CreateComponent<'a>>);

impl<'a> From<CreateComponents<'a>> for Cow<'a, [CreateComponent<'a>]> {
    fn from(value: CreateComponents<'a>) -> Self {
        Cow::Owned(value.0)
    }
}

impl<'a> From<Vec<CreateComponent<'a>>> for CreateComponents<'a> {
    fn from(value: Vec<CreateComponent<'a>>) -> Self {
        Self(value)
    }
}

impl<'a> From<CreateComponents<'a>> for Vec<CreateComponent<'a>> {
    fn from(value: CreateComponents<'a>) -> Self {
        value.into_inner()
    }
}

impl<'a> Deref for CreateComponents<'a> {
    type Target = Vec<CreateComponent<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CreateComponents<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> CreateComponents<'a> {
    /// Creates a new, empty [`CreateComponents`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new, empty [`CreateComponents`] with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Pushes a new component to the collection.
    pub fn push(&mut self, component: impl IntoComponent<'a>) {
        self.0.push(component.into_component());
    }

    /// Gets the inner [`Vec`].
    pub fn into_inner(self) -> Vec<CreateComponent<'a>> {
        self.0
    }
}

impl<'a, A: IntoComponent<'a>> FromIterator<A> for CreateComponents<'a> {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        Self(iter.into_iter().map(A::into_component).collect())
    }
}

pub trait IntoComponent<'a> {
    fn into_component(self) -> CreateComponent<'a>;
}

impl<'a> IntoComponent<'a> for CreateComponent<'a> {
    fn into_component(self) -> Self {
        self
    }
}

pub trait IntoSectionComponent<'a> {
    fn into_section_component(self) -> CreateSectionComponent<'a>;
}

impl<'a> IntoSectionComponent<'a> for CreateSectionComponent<'a> {
    fn into_section_component(self) -> Self {
        self
    }
}

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

macro_rules! impl_text_into_component {
    ($Ty:ty) => {
        impl<'a> IntoComponent<'a> for $Ty {
            fn into_component(self) -> CreateComponent<'a> {
                CreateComponent::TextDisplay(CreateTextDisplay::new(self))
            }
        }

        impl<'a> IntoSectionComponent<'a> for $Ty {
            fn into_section_component(self) -> CreateSectionComponent<'a> {
                CreateSectionComponent::TextDisplay(CreateTextDisplay::new(self))
            }
        }
    };
}

impl_text_into_component!(&'a str);
impl_text_into_component!(String);
impl_text_into_component!(&'a String);
impl_text_into_component!(Cow<'a, str>);
impl_text_into_component!(FixedString<u8>);
impl_text_into_component!(&'a FixedString<u8>);
impl_text_into_component!(FixedString<u16>);
impl_text_into_component!(&'a FixedString<u16>);
impl_text_into_component!(FixedString<u32>);
impl_text_into_component!(&'a FixedString<u32>);

/// # Examples
///
/// ```
/// _ = components![];
/// ```
macro_rules! components {
    [$($e:expr),* $(,)?] => {
        $crate::helper::discord::components::CreateComponents::from(::std::vec![
            $($crate::helper::discord::components::IntoComponent::into_component($e)),*
        ])
    };
}

/// # Examples
///
/// ```
/// let _: [(); 0] = components_array![];
/// ```
macro_rules! components_array {
    [$($e:expr),* $(,)?] => {
        [
            $($crate::helper::discord::components::IntoComponent::into_component($e)),*
        ]
    };
}

/// # Examples
///
/// ```
/// _ = components_array![];
/// ```
macro_rules! section_components {
    [$($e:expr),* $(,)?] => {{
        let v: ::std::vec::Vec<::serenity::builder::CreateSectionComponent<'_>> = ::std::vec![
            $($crate::helper::discord::components::IntoSectionComponent::into_section_component($e)),*
        ];
        v
    }};
}

pub(crate) use {components, components_array, section_components};
