use std::ops::{Deref, DerefMut};

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

/// Marker trait for [`CreateComponent`] and co.
pub trait AnyComponent {}
impl AnyComponent for CreateComponent<'_> {}
impl AnyComponent for CreateContainerComponent<'_> {}
impl AnyComponent for CreateSectionComponent<'_> {}
impl AnyComponent for CreateModalComponent<'_> {}

/// Provides an infallible conversion to [`CreateComponent`].
pub trait IntoComponent<T: AnyComponent> {
    /// Converts this value to a component.
    fn into_component(self) -> T;
}

impl<T: AnyComponent> IntoComponent<T> for T {
    fn into_component(self) -> T {
        self
    }
}

/// A collection of [`CreateComponent`]s.
///
/// This is a thin wrapper around a [`Vec`] and derefs to it.
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct ComponentVec<T: AnyComponent>(Vec<T>);

// so long and short: this wrapper exists for the `push` method and `Extend`
// impls, purely for convenience.
impl<T: AnyComponent> ComponentVec<T> {
    /// Creates a new, empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new, empty collection with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Pushes a new component to the collection.
    pub fn push<C: IntoComponent<T>>(&mut self, component: C) {
        self.0.push(component.into_component());
    }

    /// Gets the inner [`Vec`].
    pub fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<T: AnyComponent> Default for ComponentVec<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T: AnyComponent + Clone> From<ComponentVec<T>> for Cow<'_, [T]> {
    fn from(value: ComponentVec<T>) -> Self {
        Cow::Owned(value.0)
    }
}

impl<T: AnyComponent> From<Vec<T>> for ComponentVec<T> {
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}

impl<T: AnyComponent> From<ComponentVec<T>> for Vec<T> {
    fn from(value: ComponentVec<T>) -> Self {
        value.into_inner()
    }
}

impl<A: AnyComponent, U: IntoComponent<A>> Extend<U> for ComponentVec<A> {
    fn extend<T: IntoIterator<Item = U>>(&mut self, iter: T) {
        self.0.extend(iter.into_iter().map(U::into_component))
    }
}

impl<A: AnyComponent, U: IntoComponent<A>> FromIterator<U> for ComponentVec<A> {
    fn from_iter<T: IntoIterator<Item = U>>(iter: T) -> Self {
        Self(iter.into_iter().map(U::into_component).collect())
    }
}

impl<T: AnyComponent> Deref for ComponentVec<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: AnyComponent> DerefMut for ComponentVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Creates a [`ComponentVec`] from a set of [`IntoComponent`] items.
///
/// # Examples
///
/// ```
/// # use crate::helper::discord::{components, ComponentVec};
/// # use serenity::builder::*;
/// let comps: ComponentVec<CreateComponent<'_>> = components![
///     CreateTextDisplay::new("hello")
/// ];
/// # _ = comps;
/// ```
macro_rules! components {
    [$($e:expr),* $(,)?] => {
        $crate::helper::discord::components::ComponentVec::from(::std::vec![
            $($crate::helper::discord::components::IntoComponent::into_component($e)),*
        ])
    };
}

/// Creates an array from a set of [`IntoComponent`] items.
///
/// # Examples
///
/// ```
/// # use crate::helper::discord::{components_array, ComponentVec};
/// # use serenity::builder::*;
/// let comps: [CreateComponent<'_>; 1] = components_array![
///     CreateTextDisplay::new("hello")
/// ];
/// # _ = comps;
/// ```
macro_rules! components_array {
    [$($e:expr),* $(,)?] => {
        [
            $($crate::helper::discord::components::IntoComponent::into_component($e)),*
        ]
    };
}

pub(crate) use {components, components_array};

mod impls {
    use super::IntoComponent;
    use crate::prelude::*;

    macro_rules! impl_into_component {
        ($Ty:ty, $A:ty, $var:ident) => {
            impl<'a> IntoComponent<$A> for $Ty {
                fn into_component(self) -> $A {
                    <$A>::$var(self)
                }
            }
        };
    }

    impl_into_component!(CreateActionRow<'a>, CreateComponent<'a>, ActionRow);
    impl_into_component!(CreateSection<'a>, CreateComponent<'a>, Section);
    impl_into_component!(CreateTextDisplay<'a>, CreateComponent<'a>, TextDisplay);
    impl_into_component!(CreateMediaGallery<'a>, CreateComponent<'a>, MediaGallery);
    impl_into_component!(CreateFile<'a>, CreateComponent<'a>, File);
    impl_into_component!(CreateSeparator, CreateComponent<'a>, Separator);
    impl_into_component!(CreateContainer<'a>, CreateComponent<'a>, Container);
    impl_into_component!(CreateLabel<'a>, CreateComponent<'a>, Label);

    impl_into_component!(CreateActionRow<'a>, CreateContainerComponent<'a>, ActionRow);
    impl_into_component!(CreateSection<'a>, CreateContainerComponent<'a>, Section);
    impl_into_component!(
        CreateTextDisplay<'a>,
        CreateContainerComponent<'a>,
        TextDisplay
    );
    impl_into_component!(
        CreateMediaGallery<'a>,
        CreateContainerComponent<'a>,
        MediaGallery
    );
    impl_into_component!(CreateFile<'a>, CreateContainerComponent<'a>, File);
    impl_into_component!(CreateSeparator, CreateContainerComponent<'a>, Separator);

    impl_into_component!(
        CreateTextDisplay<'a>,
        CreateSectionComponent<'a>,
        TextDisplay
    );

    impl_into_component!(CreateTextDisplay<'a>, CreateModalComponent<'a>, TextDisplay);
    impl_into_component!(CreateLabel<'a>, CreateModalComponent<'a>, Label);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components() {
        let comps = components![CreateTextDisplay::new("hello")];
        assert!(matches!(
            comps.as_slice(),
            [CreateComponent::TextDisplay(_)]
        ));
    }

    #[test]
    fn components_array() {
        let comps = components_array![CreateTextDisplay::new("hello")];
        assert!(matches!(
            comps.as_slice(),
            [CreateComponent::TextDisplay(_)]
        ));
    }
}
