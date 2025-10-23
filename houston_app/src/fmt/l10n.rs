#[derive(Clone, Copy, Default)]
#[expect(non_camel_case_types)]
pub enum Locale {
    #[default]
    en,
}

impl Locale {
    /// Gets the locale for the current request.
    pub fn request_locale() -> Self {
        // if we localize this bot later, we'll need an async-local
        Self::en
    }
}

macro_rules! include_resources {
    ( $name:ident: $($t:tt)* ) => {
        #[::fluent_comp::bundle(locales = $crate::fmt::l10n::Locale, default = "en", $($t)*)]
        pub struct $name;

        impl $name {
            /// Gets the resources with the locale for the current request.
            pub fn request_locale() -> Self {
                Self::new($crate::fmt::l10n::Locale::request_locale())
            }
        }
    };
}

pub(crate) use include_resources;
