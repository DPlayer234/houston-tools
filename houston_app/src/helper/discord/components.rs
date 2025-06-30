use crate::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct CreateComponents<'a> {
    pub components: Vec<CreateComponent<'a>>,
}

impl<'a> From<CreateComponents<'a>> for Cow<'a, [CreateComponent<'a>]> {
    fn from(value: CreateComponents<'a>) -> Self {
        Cow::Owned(value.components)
    }
}

impl<'a> CreateComponents<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            components: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, component: impl IntoComponent<'a>) {
        self.components.push(component.into_component());
    }
}

impl<'a, A: IntoComponent<'a>> FromIterator<A> for CreateComponents<'a> {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        Self {
            components: iter.into_iter().map(A::into_component).collect(),
        }
    }
}

pub trait IntoComponent<'a> {
    fn into_component(self) -> CreateComponent<'a>;
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

impl_into_component!(CreateActionRow<'a>, ActionRow);
impl_into_component!(CreateSection<'a>, Section);
impl_into_component!(CreateTextDisplay<'a>, TextDisplay);
impl_into_component!(CreateMediaGallery<'a>, MediaGallery);
impl_into_component!(CreateFile<'a>, File);
impl_into_component!(CreateSeparator, Separator);
impl_into_component!(CreateContainer<'a>, Container);

macro_rules! components {
    [$($e:expr),* $(,)?] => {
        $crate::helper::discord::CreateComponents {
            components: ::std::vec![
                $($crate::helper::discord::IntoComponent::into_component($e)),*
            ]
        }
    };
}

pub(crate) use components;
