use darling::util::PathList;
use syn::{Ident, Path};

#[derive(Debug, darling::FromMeta)]
#[darling(allow_unknown_fields)]
pub struct FieldMeta {
    #[darling(multiple)]
    pub serde: Vec<FieldSerdeMeta>,
}

#[derive(Default, Debug, darling::FromMeta)]
#[darling(allow_unknown_fields)]
pub struct FieldSerdeMeta {
    pub rename: Option<Ident>,
    pub with: Option<Path>,
    pub serialize_with: Option<Path>,
}

#[derive(Debug, darling::FromDeriveInput)]
#[darling(attributes(model))]
pub struct ModelMeta {
    pub derive: Option<PathList>,
}

impl FieldSerdeMeta {
    pub fn has_with(&self) -> bool {
        self.with.is_some() || self.serialize_with.is_some()
    }

    pub fn merge(mut many: Vec<Self>) -> Self {
        let mut result = many.pop().unwrap_or_default();
        for item in many {
            if item.rename.is_some() {
                result.rename = item.rename;
            }
            if item.with.is_some() {
                result.with = item.with;
            }
            if item.serialize_with.is_some() {
                result.serialize_with = item.serialize_with;
            }
        }
        result
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
    pub partial_name: syn::Ident,
    pub filter_name: syn::Ident,
    pub sort_name: syn::Ident,
    pub fields_name: syn::Ident,
    pub internals_name: syn::Ident,
    pub fields: Vec<FieldArgs<'a>>,
    pub derive: &'a [Path],
}
