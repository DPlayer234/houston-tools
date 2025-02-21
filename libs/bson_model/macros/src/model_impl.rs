use darling::ast::NestedMeta;
use darling::{FromDeriveInput as _, FromMeta as _};
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident};
use syn::{Data, Fields};

use crate::args::{FieldArgs, FieldMeta, FieldSerdeMeta, ModelArgs, ModelMeta};

pub fn entry_point(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let model_meta = ModelMeta::from_derive_input(&input)?;

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

    let mut parsed_fields = Vec::new();

    for field in &mut fields.named {
        let args: Vec<_> = field
            .attrs
            .drain(..)
            .map(|attr| NestedMeta::Meta(attr.meta))
            .collect();
        let args = FieldMeta::from_list(&args)?;
        let args = FieldSerdeMeta::merge(args.serde);

        // exclude non-serialized fields from the output
        if args.has_skip() {
            continue;
        }

        let Some(ident) = field.ident.as_ref() else {
            return Err(syn::Error::new_spanned(
                field,
                "all fields must have a name",
            ));
        };

        parsed_fields.push(FieldArgs {
            name: ident,
            ty: &field.ty,
            args,
        });
    }

    let default_derive = [
        syn::parse_quote!(::std::fmt::Debug),
        syn::parse_quote!(::std::clone::Clone),
    ];

    let args = ModelArgs {
        vis: &input.vis,
        ty_name: &input.ident,
        partial_name: format_ident!("{}Partial", input.ident),
        filter_name: format_ident!("{}Filter", input.ident),
        sort_name: format_ident!("{}Sort", input.ident),
        fields_name: format_ident!("{}Fields", input.ident),
        internals_name: format_ident!("__{}_model_document_internals", input.ident),
        fields: parsed_fields,
        derive: model_meta
            .derive
            .as_deref()
            .map(Vec::as_slice)
            .unwrap_or(&default_derive),
    };

    let internals = emit_internals(&args);
    let update = emit_partial(&args);
    let filter = emit_filter(&args);
    let sort = emit_sort(&args);
    let fields = emit_fields(&args);

    Ok(quote::quote! {
        #internals
        #update
        #filter
        #sort
        #fields
    })
}

fn emit_internals(args: &ModelArgs<'_>) -> TokenStream {
    let ModelArgs {
        ty_name,
        partial_name,
        filter_name,
        sort_name,
        fields_name,
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
                pub(super) fn #update_with_name<S>(field: &::std::option::Option<#ty>, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                where
                    S: bson_model::private::serde::ser::Serializer,
                {
                    match field {
                        ::std::option::Option::Some(value) => #source_with(value, serializer),
                        ::std::option::Option::None => serializer.serialize_none(),
                    }
                }

                pub(super) fn #filter_with_name<S>(field: &::std::option::Option<bson_model::Filter<#ty>>, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                where
                    S: bson_model::private::serde::ser::Serializer,
                {
                    struct With;
                    impl bson_model::private::SerdeWith<#ty> for With {
                        fn serialize<S>(&self, value: &#ty, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                        where
                            S: bson_model::private::serde::Serializer,
                        {
                            #source_with(value, serializer)
                        }
                    }

                    match field {
                        ::std::option::Option::Some(value) => bson_model::private::serialize_filter_with(value, serializer, With),
                        ::std::option::Option::None => serializer.serialize_none(),
                    }
                }
            }
        });

    quote::quote! {
        #[automatically_derived]
        impl bson_model::ModelDocument for #ty_name {
            type Partial = #partial_name;
            type Filter = #filter_name;
            type Sort = #sort_name;
            type Fields = #fields_name;

            fn partial() -> #partial_name {
                #partial_name::new()
            }

            fn filter() -> #filter_name {
                #filter_name::new()
            }

            fn sort() -> #sort_name {
                #sort_name::new()
            }

            fn fields() -> #fields_name {
                #fields_name (())
            }
        }

        /// Not intended for use. Implementation detail of the `bson_model` macro expansion.
        #[doc(hidden)]
        #[allow(non_snake_case, clippy::ref_option)]
        mod #internals_name {
            use super::*;
            #( #field_methods )*
        }
    }
}

fn emit_partial(args: &ModelArgs<'_>) -> TokenStream {
    let ModelArgs {
        vis,
        ty_name,
        partial_name,
        internals_name,
        fields,
        derive,
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

    let into_document = emit_into_document(partial_name);

    quote::quote! {
        #[doc = concat!("A partial [`", stringify!(#ty_name), "`].")]
        #[derive(::std::default::Default, bson_model::private::serde::Serialize #(,#derive)*)]
        #[non_exhaustive]
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

        #into_document
    }
}

fn emit_filter(args: &ModelArgs<'_>) -> TokenStream {
    let ModelArgs {
        vis,
        ty_name,
        internals_name,
        filter_name,
        fields,
        derive,
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

    let into_document = emit_into_document(filter_name);

    quote::quote! {
        #[doc = concat!("A filter builder for [`", stringify!(#ty_name), "`].")]
        #[derive(::std::default::Default, bson_model::private::serde::Serialize #(, #derive)*)]
        #[non_exhaustive]
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

        #into_document
    }
}

fn emit_sort(args: &ModelArgs<'_>) -> TokenStream {
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
                self.0.insert(#rename, #name);
                self
            }
        }
    });

    quote::quote! {
        #[doc = concat!("A sort builder for [`", stringify!(#ty_name), "`].")]
        ///
        /// This represents a thin wrapper around a [`Document`](bson_model::private::bson::Document) to retain the used sort priority.
        #[derive(::std::default::Default, ::std::fmt::Debug, ::std::clone::Clone, ::std::cmp::PartialEq, bson_model::private::serde::Serialize)]
        #[serde(transparent)]
        #vis struct #sort_name(bson_model::private::bson::Document);

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
                self.0
            }
        }

        #[automatically_derived]
        impl From<#sort_name> for bson_model::private::bson::Document {
            fn from(value: #sort_name) -> Self {
                value.into_document()
            }
        }
    }
}

fn emit_fields(args: &ModelArgs<'_>) -> TokenStream {
    let ModelArgs {
        vis,
        fields_name,
        fields,
        ..
    } = args;

    let field_methods = fields.iter().map(|field| {
        let FieldArgs { name, args, .. } = field;
        let rename = args.rename.as_ref().unwrap_or(name).to_string();
        let expr_name = "$".to_owned() + &rename;

        quote::quote! {
            #[doc = concat!("Gets the BSON `", stringify!(#name), "` field.")]
            pub const fn #name(self) -> bson_model::ModelField {
                const {
                    bson_model::ModelField::new(#expr_name)
                }
            }
        }
    });

    quote::quote! {
        #[derive(::std::fmt::Debug, ::std::clone::Clone, ::std::marker::Copy)]
        #vis struct #fields_name(());

        impl #fields_name {
            #( #field_methods )*
        }
    }
}

fn emit_into_document(ty_name: impl ToTokens) -> TokenStream {
    quote::quote! {
        impl #ty_name {
            /// Tries to serialize this value into a BSON document.
            pub fn into_document(self) -> bson_model::private::bson::ser::Result<bson_model::private::bson::Document> {
                bson_model::private::bson::to_document(&self)
            }
        }

        #[automatically_derived]
        impl TryFrom<#ty_name> for bson_model::private::bson::Document {
            type Error = bson_model::private::bson::ser::Error;

            fn try_from(value: #ty_name) -> Result<Self, Self::Error> {
                value.into_document()
            }
        }
    }
}
