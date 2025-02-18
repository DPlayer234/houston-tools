use darling::ast::NestedMeta;
use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::format_ident;
use syn::{Data, Fields};

use crate::args::{FieldArgs, FieldSerdeMeta, FieldSerdeMetaOuter, ModelArgs};

pub fn entry_point(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let Data::Struct(data) = input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "ModelDocument must be applied to a struct with named fields",
        ));
    };

    let Fields::Named(mut fields) = data.fields else {
        return Err(syn::Error::new_spanned(
            data.fields,
            "ModelDocument must be applied to a struct with named fields",
        ));
    };

    let mut parsed = Vec::new();

    for field in &mut fields.named {
        let Some(ident) = field.ident.as_ref() else {
            return Err(syn::Error::new_spanned(
                field,
                "all fields must have a name",
            ));
        };

        let attrs: Vec<_> = field
            .attrs
            .drain(..)
            .map(|attr| NestedMeta::Meta(attr.meta))
            .collect();
        let attrs = FieldSerdeMetaOuter::from_list(&attrs)?;

        parsed.push(FieldArgs {
            name: ident,
            ty: &field.ty,
            args: FieldSerdeMeta::merge(attrs.serde),
        });
    }

    let args = ModelArgs {
        vis: &input.vis,
        ty_name: &input.ident,
        partial_name: format_ident!("{}Partial", input.ident),
        filter_name: format_ident!("{}Filter", input.ident),
        sort_name: format_ident!("{}Sort", input.ident),
        internals_name: format_ident!("__{}Internals", input.ident),
        fields: parsed,
    };

    let internals = emit_internals(&args)?;
    let update = emit_partial(&args)?;
    let filter = emit_filter(&args)?;
    let sort = emit_sort(&args)?;

    Ok(quote::quote! {
        #internals
        #update
        #filter
        #sort
    })
}

fn emit_internals(args: &ModelArgs<'_>) -> syn::Result<TokenStream> {
    let ModelArgs {
        ty_name,
        partial_name,
        filter_name,
        sort_name,
        internals_name,
        fields,
        ..
    } = args;

    let field_methods = fields.iter()
        .filter(|field| field.args.has_with())
        .map(|field| {
            let FieldArgs { name, ty, args } = field;
            let FieldSerdeMeta { with, serialize_with, .. } = args;

            let update_with_name = format_ident!("partial_{}", name);
            let filter_with_name = format_ident!("filter_{}", name);
            let source_with = with
                .as_ref()
                .map(|w| quote::quote! { #w::serialize })
                .unwrap_or_else(|| {
                    let w = serialize_with.as_ref().expect("must be specified");
                    quote::quote! { #w }
                });

            quote::quote! {
                pub(super) fn #update_with_name<S>(field: &::std::option::Option<#ty>, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: bson_model::private::serde::ser::Serializer,
                {
                    match field {
                        ::std::option::Option::Some(value) => #source_with(value, serializer),
                        ::std::option::Option::None => serializer.serialize_none(),
                    }
                }

                pub(super) fn #filter_with_name<S>(field: &::std::option::Option<bson_model::Filter<#ty>>, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: bson_model::private::serde::ser::Serializer,
                {
                    struct With;
                    impl bson_model::private::SerdeWith<#ty> for With {
                        fn serialize<S>(&self, value: &#ty, serializer: S) -> Result<S::Ok, S::Error>
                        where
                            S: bson_model::private::serde::Serializer,
                        {
                            #source_with(value, serializer)
                        }
                    }

                    match field {
                        ::std::option::Option::Some(value) => bson_model::private::wrap_filter_with(value, serializer, With),
                        ::std::option::Option::None => serializer.serialize_none(),
                    }
                }
            }
        });

    Ok(quote::quote! {
        #[automatically_derived]
        impl bson_model::ModelDocument for #ty_name {
            type Partial = #partial_name;
            type Filter = #filter_name;
            type Sort = #sort_name;

            fn partial() -> #partial_name {
                #partial_name::new()
            }

            fn filter() -> #filter_name {
                #filter_name::new()
            }

            fn sort() -> #sort_name {
                #sort_name::new()
            }
        }

        /// Not intended for use. Implementation detail of the `bson_model` macro expansion.
        #[doc(hidden)]
        #[allow(non_snake_case, clippy::ref_option)]
        mod #internals_name {
            use super::*;
            #( #field_methods )*
        }
    })
}

fn emit_partial(args: &ModelArgs<'_>) -> syn::Result<TokenStream> {
    let ModelArgs {
        vis,
        ty_name,
        partial_name,
        internals_name,
        fields,
        ..
    } = args;

    let field_decls = fields.iter().map(|field| {
        let FieldArgs { name, ty, args } = field;
        let with = if args.has_with() {
            Some(format!("{internals_name}::partial_{name}"))
        } else {
            None
        }
        .into_iter();

        let rename = args.rename.as_ref().map(ToString::to_string).into_iter();

        quote::quote! {
            #(#[serde(serialize_with = #with)])*
            #(#[serde(rename = #rename)])*
            #[serde(skip_serializing_if = "::std::option::Option::is_none")]
            pub #name: ::std::option::Option<#ty>,
        }
    });

    let field_methods = fields.iter().map(|field| {
        let FieldArgs { name, ty, .. } = field;

        quote::quote! {
            #[doc = concat!("Sets the `", stringify!(#name), "` field.")]
            #[must_use]
            pub fn #name(mut self, #name: #ty) -> Self {
                self.#name = ::std::option::Option::Some(#name);
                self
            }
        }
    });

    Ok(quote::quote! {
        #[doc = concat!("A partial [`", stringify!(#ty_name), "`].")]
        #[derive(Default, Debug, Clone, PartialEq, bson_model::private::serde::Serialize)]
        #vis struct #partial_name {
            #( #field_decls )*
        }

        impl #partial_name {
            /// Create a new value.
            #[must_use]
            pub fn new() -> Self {
                ::std::default::Default::default()
            }

            #( #field_methods )*
        }

        impl #partial_name {
            /// Tries to serialize this value into a BSON document.
            pub fn into_document(self) -> bson_model::private::bson::ser::Result<bson_model::private::bson::Document> {
                bson_model::private::bson::to_document(&self)
            }
        }

        #[automatically_derived]
        impl TryFrom<#partial_name> for bson_model::private::bson::Document {
            type Error = bson_model::private::bson::ser::Error;

            fn try_from(value: #partial_name) -> Result<Self, Self::Error> {
                value.into_document()
            }
        }
    })
}

fn emit_filter(args: &ModelArgs<'_>) -> syn::Result<TokenStream> {
    let ModelArgs {
        vis,
        ty_name,
        internals_name,
        filter_name,
        fields,
        ..
    } = args;

    let field_decls = fields.iter().map(|field| {
        let FieldArgs { name, ty, args } = field;
        let with = if args.has_with() {
            Some(format!("{internals_name}::filter_{name}"))
        } else {
            None
        }
        .into_iter();

        let rename = args.rename.as_ref().map(ToString::to_string).into_iter();

        quote::quote! {
            #(#[serde(serialize_with = #with)])*
            #(#[serde(rename = #rename)])*
            #[serde(skip_serializing_if = "::std::option::Option::is_none")]
            pub #name: ::std::option::Option<bson_model::Filter<#ty>>,
        }
    });

    let field_methods = fields.iter().map(|field| {
        let FieldArgs { name, ty, .. } = field;

        quote::quote! {
            #[doc = concat!("Sets the filter condition for the `", stringify!(#name), "` field.")]
            #[must_use]
            pub fn #name(mut self, #name: impl ::std::convert::Into<bson_model::Filter<#ty>>) -> Self {
                self.#name = Some(::std::convert::Into::into(#name));
                self
            }
        }
    });

    Ok(quote::quote! {
        #[doc = concat!("A filter builder for [`", stringify!(#ty_name), "`].")]
        #[derive(Default, Debug, Clone, PartialEq, bson_model::private::serde::Serialize)]
        #vis struct #filter_name {
            #( #field_decls )*
        }

        impl #filter_name {
            /// Create a new value.
            #[must_use]
            pub fn new() -> Self {
                ::std::default::Default::default()
            }

            #( #field_methods )*
        }

        impl #filter_name {
            /// Tries to serialize this value into a BSON document.
            pub fn into_document(self) -> bson_model::private::bson::ser::Result<bson_model::private::bson::Document> {
                bson_model::private::bson::to_document(&self)
            }
        }

        #[automatically_derived]
        impl TryFrom<#filter_name> for bson_model::private::bson::Document {
            type Error = bson_model::private::bson::ser::Error;

            fn try_from(value: #filter_name) -> Result<Self, Self::Error> {
                value.into_document()
            }
        }
    })
}

fn emit_sort(args: &ModelArgs<'_>) -> syn::Result<TokenStream> {
    let ModelArgs {
        vis,
        ty_name,
        sort_name,
        fields,
        ..
    } = args;

    let field_methods = fields.iter().map(|field| {
        let FieldArgs { name, args, .. } = field;
        let rename = args.rename.as_ref().unwrap_or(name).to_string();

        quote::quote! {
            #[doc = concat!("Sorts the document by the `", stringify!(#name), "` field.")]
            ///
            /// The order of function calls impacts the sort!
            #[must_use]
            pub fn #name(mut self, #name: bson_model::Sort) -> Self {
                self.doc.insert(#rename, #name);
                self
            }
        }
    });

    Ok(quote::quote! {
        #[doc = concat!("A sort builder for [`", stringify!(#ty_name), "`].")]
        #[derive(Default, Debug, Clone, PartialEq, bson_model::private::serde::Serialize)]
        #vis struct #sort_name {
            doc: bson_model::private::bson::Document,
        }

        impl #sort_name {
            /// Create a new value.
            #[must_use]
            pub fn new() -> Self {
                ::std::default::Default::default()
            }

            #( #field_methods )*
        }

        impl #sort_name {
            /// Gets the serialized BSON document.
            pub fn into_document(self) -> bson_model::private::bson::Document {
                self.doc
            }
        }

        #[automatically_derived]
        impl From<#sort_name> for Document {
            fn from(value: #sort_name) -> Self {
                value.into_document()
            }
        }
    })
}
