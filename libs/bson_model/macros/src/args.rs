use darling::util::{Flag, PathList};
use syn::{Generics, Ident, Path};

#[derive(Default, Debug, darling::FromAttributes)]
#[darling(attributes(serde))]
#[darling(allow_unknown_fields)]
pub struct FieldSerdeMeta {
    pub rename: Option<Ident>,
    pub with: Option<Path>,
    pub serialize_with: Option<Path>,
    // - not checking for conditional skips because this code shouldn't accidentally exclude those
    //   from updates/filters (plus it'd be extra effort to)
    // - not checking `skip_deserializing` because the data might not be deserializable but
    //   serializable and relevant for updates/filters
    pub skip: Flag,
    pub skip_serializing: Flag,
}

#[derive(Debug, darling::FromDeriveInput)]
#[darling(attributes(model))]
pub struct ModelMeta {
    #[darling(rename = "crate")]
    pub crate_: Option<Path>,
    pub derive_partial: Option<PathList>,
    pub derive_filter: Option<PathList>,
}

impl FieldSerdeMeta {
    pub fn has_with(&self) -> bool {
        self.with.is_some() || self.serialize_with.is_some()
    }

    pub fn has_skip(&self) -> bool {
        self.skip.is_present() || self.skip_serializing.is_present()
    }
}

pub struct FieldArgs<'a> {
    pub name: &'a syn::Ident,
    pub ty: &'a syn::Type,
    pub args: FieldSerdeMeta,
}

pub struct ModelArgs<'a> {
    pub vis: &'a syn::Visibility,
    pub ty_name: &'a syn::Ident,
    pub generics: &'a Generics,
    pub partial_name: syn::Ident,
    pub filter_name: syn::Ident,
    pub sort_name: syn::Ident,
    pub fields_name: syn::Ident,
    pub internals_name: syn::Ident,
    pub fields: Vec<FieldArgs<'a>>,
    pub crate_: Path,
    pub derive_partial: &'a [Path],
    pub derive_filter: &'a [Path],
}
