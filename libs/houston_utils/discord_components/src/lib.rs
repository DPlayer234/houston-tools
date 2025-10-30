use std::borrow::Cow;
use std::ops::{Deref, DerefMut};

use serenity::builder::*;

mod impls;
#[cfg(test)]
mod tests;

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

macro_rules! define_component_builder {
    ($Ident:ident, $Trait:ident, $convert:ident, $Wrap:ident) => {
        #[doc = concat!("A collection of [`", stringify!($Wrap), "`]s.")]
        ///
        /// This is a thin wrapper around a [`Vec`] and derefs to it.
        #[derive(Debug, Clone, Default)]
        #[repr(transparent)]
        pub struct $Ident<'a>(Vec<$Wrap<'a>>);

        #[doc = concat!("Provides an infallible conversion to [`", stringify!($Wrap), "`].")]
        pub trait $Trait<'a> {
            /// Converts this value to a component.
            fn $convert(self) -> $Wrap<'a>;
        }

        impl<'a> $Trait<'a> for $Wrap<'a> {
            fn $convert(self) -> Self {
                self
            }
        }

        impl<'a> $Ident<'a> {
            /// Creates a new, empty collection.
            pub fn new() -> Self {
                Self::default()
            }

            /// Creates a new, empty collection with the specified capacity.
            pub fn with_capacity(capacity: usize) -> Self {
                Self(Vec::with_capacity(capacity))
            }

            /// Pushes a new component to the collection.
            pub fn push(&mut self, component: impl $Trait<'a>) {
                self.0.push(component.$convert());
            }

            /// Gets the inner [`Vec`].
            pub fn into_inner(self) -> Vec<$Wrap<'a>> {
                self.0
            }
        }

        impl<'a> From<$Ident<'a>> for Cow<'a, [$Wrap<'a>]> {
            fn from(value: $Ident<'a>) -> Self {
                Cow::Owned(value.0)
            }
        }

        impl<'a> From<Vec<$Wrap<'a>>> for $Ident<'a> {
            fn from(value: Vec<$Wrap<'a>>) -> Self {
                Self(value)
            }
        }

        impl<'a> From<$Ident<'a>> for Vec<$Wrap<'a>> {
            fn from(value: $Ident<'a>) -> Self {
                value.into_inner()
            }
        }

        impl<'a, A: $Trait<'a>> Extend<A> for $Ident<'a> {
            fn extend<T: IntoIterator<Item = A>>(&mut self, iter: T) {
                self.0.extend(iter.into_iter().map(A::$convert))
            }
        }

        impl<'a, A: $Trait<'a>> FromIterator<A> for $Ident<'a> {
            fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
                Self(iter.into_iter().map(A::$convert).collect())
            }
        }

        impl<'a> Deref for $Ident<'a> {
            type Target = Vec<$Wrap<'a>>;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for $Ident<'_> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}

define_component_builder!(
    CreateComponents,
    IntoComponent,
    into_component,
    CreateComponent
);

define_component_builder!(
    CreateSectionComponents,
    IntoSectionComponent,
    into_section_component,
    CreateSectionComponent
);

/// Creates a [`CreateComponents`] from a set of [`IntoComponent`] items.
///
/// # Examples
///
/// ```
/// # use crate::helper::discord::components;
/// let comps = components![
///     serenity::builder::CreateTextDisplay::new("hello")
/// ];
/// # _ = comps;
/// ```
#[macro_export]
macro_rules! components {
    [$($e:expr),* $(,)?] => {
        $crate::CreateComponents::from(::std::vec![
            $($crate::IntoComponent::into_component($e)),*
        ])
    };
}

/// Creates an array from a set of [`IntoComponent`] items.
///
/// # Examples
///
/// ```
/// # use crate::helper::discord::components_array;
/// let comps = components_array![
///     serenity::builder::CreateTextDisplay::new("hello")
/// ];
/// # _ = comps;
/// ```
#[macro_export]
macro_rules! components_array {
    [$($e:expr),* $(,)?] => {
        [
            $($crate::IntoComponent::into_component($e)),*
        ]
    };
}

/// Creates a [`CreateSectionComponents`] from a set of [`IntoSectionComponent`]
/// items.
///
/// # Examples
///
/// ```
/// # use crate::helper::discord::section_components;
/// let comps = section_components![
///     serenity::builder::CreateTextDisplay::new("hello")
/// ];
/// # _ = comps;
/// ```
#[macro_export]
macro_rules! section_components {
    [$($e:expr),* $(,)?] => {
        $crate::CreateSectionComponents::from(::std::vec![
            $($crate::IntoSectionComponent::into_section_component($e)),*
        ])
    };
}
