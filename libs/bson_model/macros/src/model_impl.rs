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
        generics: &input.generics,
        partial_name: format_ident!("{}Partial", input.ident),
        filter_name: format_ident!("{}Filter", input.ident),
        sort_name: format_ident!("{}Sort", input.ident),
        fields_name: format_ident!("{}Fields", input.ident),
        internals_name: format_ident!("__{}_model_document_internals", input.ident),
        fields: parsed_fields,
        crate_: model_meta
            .crate_
            .unwrap_or_else(|| syn::parse_quote!(::bson_model)),
        derive_partial: model_meta
            .derive_partial
            .as_deref()
            .map(Vec::as_slice)
            .unwrap_or(&default_derive),
        derive_filter: model_meta
            .derive_filter
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
        generics,
        partial_name,
        filter_name,
        sort_name,
        fields_name,
        internals_name,
        fields,
        crate_,
        ..
    } = args;

    let (impl_gen, ty_gen, where_clause) = generics.split_for_impl();
    let turbo_fish = ty_gen.as_turbofish();

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
                fn #update_with_name<S>(field: &::std::option::Option<#ty>, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                where
                    S: #crate_::private::serde::ser::Serializer,
                {
                    match field {
                        ::std::option::Option::Some(value) => #source_with(value, serializer),
                        ::std::option::Option::None => serializer.serialize_none(),
                    }
                }

                fn #filter_with_name<S>(field: &::std::option::Option<#crate_::Filter<#ty>>, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                where
                    S: #crate_::private::serde::ser::Serializer,
                {
                    struct With #impl_gen (::std::marker::PhantomData<#ty_name #ty_gen>) #where_clause;

                    impl #impl_gen #crate_::private::SerdeWith<#ty> for With #ty_gen #where_clause {
                        fn serialize<S>(&self, value: &#ty, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                        where
                            S: #crate_::private::serde::Serializer,
                        {
                            #source_with(value, serializer)
                        }
                    }

                    let with = With #turbo_fish (::std::marker::PhantomData);
                    match field {
                        ::std::option::Option::Some(value) => #crate_::private::serialize_filter_with(value, serializer, with),
                        ::std::option::Option::None => serializer.serialize_none(),
                    }
                }
            }
        });

    quote::quote! {
        #[automatically_derived]
        impl #impl_gen #crate_::ModelDocument for #ty_name #ty_gen #where_clause {
            type Partial = #partial_name #ty_gen;
            type Filter = #filter_name #ty_gen;
            type Sort = #sort_name #ty_gen;
            type Fields = #fields_name;

            fn partial() -> Self::Partial {
                #partial_name::new()
            }

            fn filter() -> Self::Filter {
                #filter_name::new()
            }

            fn sort() -> Self::Sort {
                #sort_name::new()
            }

            fn fields() -> Self::Fields {
                #fields_name (())
            }
        }

        /// Not intended for use. Implementation detail of the `bson_model` macro expansion.
        #[doc(hidden)]
        #[allow(non_camel_case_types, dead_code)]
        struct #internals_name #impl_gen (::std::convert::Infallible, ::std::marker::PhantomData<#ty_name #ty_gen>) #where_clause;

        #[allow(clippy::ref_option)]
        impl #impl_gen #internals_name #ty_gen #where_clause {
            #( #field_methods )*
        }
    }
}

fn emit_partial(args: &ModelArgs<'_>) -> TokenStream {
    let ModelArgs {
        vis,
        ty_name,
        generics,
        partial_name,
        internals_name,
        fields,
        crate_,
        derive_partial,
        ..
    } = args;

    let (impl_gen, ty_gen, where_clause) = generics.split_for_impl();
    let turbo_fish = ty_gen.as_turbofish().into_token_stream().to_string();

    let field_decls = fields.iter().map(|field| {
        let FieldArgs { name, ty, args } = field;
        let with = if args.has_with() {
            Some(format!("{internals_name}{turbo_fish}::partial_{name}"))
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

    let field_defaults = fields.iter().map(|field| {
        let FieldArgs { name, .. } = field;
        quote::quote! {
            #name: None,
        }
    });

    let into_document = emit_into_document(crate_, partial_name, generics);
    let serde_crate = quote::quote!(#crate_::private::serde).to_string();

    quote::quote! {
        #[doc = concat!("A partial [`", stringify!(#ty_name), "`].")]
        #[derive(#crate_::private::serde::Serialize #(,#derive_partial)*)]
        #[serde(crate = #serde_crate)]
        #[non_exhaustive]
        #vis struct #partial_name #impl_gen #where_clause {
            #( #field_decls )*
            #[serde(skip)]
            __main_marker: ::std::marker::PhantomData<#ty_name #ty_gen>,
        }

        #[automatically_derived]
        impl #impl_gen ::std::default::Default for #partial_name #ty_gen #where_clause {
            fn default() -> Self {
                Self {
                    #( #field_defaults )*
                    __main_marker: ::std::marker::PhantomData,
                }
            }
        }

        impl #impl_gen #partial_name #ty_gen #where_clause {
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
        generics,
        internals_name,
        filter_name,
        fields,
        crate_,
        derive_filter,
        ..
    } = args;

    let (impl_gen, ty_gen, where_clause) = generics.split_for_impl();
    let turbo_fish = ty_gen.as_turbofish().into_token_stream().to_string();

    let field_decls = fields.iter().map(|field| {
        let FieldArgs { name, ty, args } = field;
        let with = if args.has_with() {
            Some(format!("{internals_name}{turbo_fish}::filter_{name}"))
        } else {
            None
        }
        .into_iter();

        let rename = args.rename.as_ref().map(ToString::to_string).into_iter();

        quote::quote! {
            #(#[serde(serialize_with = #with)])*
            #(#[serde(rename = #rename)])*
            #[serde(skip_serializing_if = "::std::option::Option::is_none")]
            pub #name: ::std::option::Option<#crate_::Filter<#ty>>,
        }
    });

    let field_methods = fields.iter().map(|field| {
        let FieldArgs { name, ty, .. } = field;

        quote::quote! {
            #[doc = concat!("Sets the filter condition for the `", stringify!(#name), "` field.")]
            #[must_use]
            pub fn #name(mut self, #name: impl ::std::convert::Into<#crate_::Filter<#ty>>) -> Self {
                self.#name = Some(::std::convert::Into::into(#name));
                self
            }
        }
    });

    let field_defaults = fields.iter().map(|field| {
        let FieldArgs { name, .. } = field;
        quote::quote! {
            #name: None,
        }
    });

    let into_document = emit_into_document(crate_, filter_name, generics);
    let serde_crate = quote::quote!(#crate_::private::serde).to_string();

    quote::quote! {
        #[doc = concat!("A filter builder for [`", stringify!(#ty_name), "`].")]
        #[derive(#crate_::private::serde::Serialize #(, #derive_filter)*)]
        #[serde(crate = #serde_crate)]
        #[non_exhaustive]
        #vis struct #filter_name #impl_gen #where_clause {
            #( #field_decls )*
            #[serde(skip)]
            __main_marker: ::std::marker::PhantomData<#ty_name #ty_gen>,
        }

        #[automatically_derived]
        impl #impl_gen ::std::default::Default for #filter_name #ty_gen #where_clause {
            fn default() -> Self {
                Self {
                    #( #field_defaults )*
                    __main_marker: ::std::marker::PhantomData,
                }
            }
        }

        impl #impl_gen #filter_name #ty_gen #where_clause {
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
        generics,
        sort_name,
        fields,
        crate_,
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
            pub fn #name(mut self, #name: #crate_::Sort) -> Self {
                self.0.insert(#rename, #name);
                self
            }
        }
    });

    let (impl_gen, ty_gen, where_clause) = generics.split_for_impl();

    quote::quote! {
        #[doc = concat!("A sort builder for [`", stringify!(#ty_name), "`].")]
        ///
        /// This represents a thin wrapper around a [`Document`] to retain the used sort priority.
        ///
        #[doc = concat!("[`Document`]: ", stringify!(#crate_), "::private::bson::Document")]
        #vis struct #sort_name #impl_gen (
            #crate_::private::bson::Document,
            ::std::marker::PhantomData<#ty_name #ty_gen>,
        ) #where_clause;

        impl #impl_gen #sort_name #ty_gen #where_clause {
            /// Create a new value.
            #[must_use]
            pub fn new() -> Self {
                ::std::default::Default::default()
            }

            #( #field_methods )*

            /// Gets the serialized BSON document.
            pub fn into_document(self) -> #crate_::private::bson::Document {
                self.0
            }
        }

        #[automatically_derived]
        impl #impl_gen ::std::default::Default for #sort_name #ty_gen #where_clause {
            fn default() -> Self {
                Self(#crate_::private::bson::Document::new(), ::std::marker::PhantomData)
            }
        }

        #[automatically_derived]
        impl #impl_gen ::std::fmt::Debug for #sort_name #ty_gen #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_tuple(stringify!(#sort_name)).field(&self.0).finish()
            }
        }

        #[automatically_derived]
        impl #impl_gen ::std::clone::Clone for #sort_name #ty_gen #where_clause {
            fn clone(&self) -> Self {
                Self(::std::clone::Clone::clone(&self.0), ::std::marker::PhantomData)
            }
        }

        #[automatically_derived]
        impl #impl_gen ::std::cmp::PartialEq for #sort_name #ty_gen #where_clause {
            fn eq(&self, other: &Self) -> bool {
                ::std::cmp::PartialEq::eq(&self.0, &other.0)
            }
        }

        #[automatically_derived]
        impl #impl_gen #crate_::private::serde::Serialize for #sort_name #ty_gen #where_clause {
            fn serialize<S: #crate_::private::serde::Serializer>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> {
                #crate_::private::serde::Serialize::serialize(&self.0, serializer)
            }
        }

        #[automatically_derived]
        impl #impl_gen From<#sort_name #ty_gen> for #crate_::private::bson::Document #where_clause {
            fn from(value: #sort_name #ty_gen) -> Self {
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
        crate_,
        ..
    } = args;

    let field_methods = fields.iter().map(|field| {
        let FieldArgs { name, args, .. } = field;
        let rename = args.rename.as_ref().unwrap_or(name).to_string();
        let expr_name = "$".to_owned() + &rename;

        quote::quote! {
            #[doc = concat!("Gets the BSON `", stringify!(#name), "` field.")]
            pub const fn #name(self) -> #crate_::ModelField {
                const {
                    #crate_::ModelField::new(#expr_name)
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

fn emit_into_document(
    crate_: &syn::Path,
    ty_name: impl ToTokens,
    generics: &syn::Generics,
) -> TokenStream {
    let (impl_gen, ty_gen, where_clause) = generics.split_for_impl();

    let mut where_clause = where_clause
        .cloned()
        .unwrap_or_else(|| syn::parse_quote!(where));

    for t in generics.type_params() {
        let ident = &t.ident;
        where_clause
            .predicates
            .push(syn::parse_quote!(#ident: #crate_::private::serde::Serialize));
    }

    quote::quote! {
        impl #impl_gen #ty_name #ty_gen #where_clause {
            /// Tries to serialize this value into a BSON document.
            pub fn into_document(self) -> #crate_::private::bson::ser::Result<#crate_::private::bson::Document> {
                #crate_::private::bson::to_document(&self)
            }
        }

        #[automatically_derived]
        impl #impl_gen TryFrom<#ty_name #ty_gen> for #crate_::private::bson::Document #where_clause {
            type Error = #crate_::private::bson::ser::Error;

            fn try_from(value: #ty_name #ty_gen) -> Result<Self, Self::Error> {
                value.into_document()
            }
        }
    }
}
