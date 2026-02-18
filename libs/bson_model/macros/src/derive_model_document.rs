use darling::{Error, FromAttributes as _, FromDeriveInput as _};
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident};
use syn::{Data, Fields, FieldsNamed, GenericParam};

use crate::args::{FieldArgs, FieldMeta, FieldSerdeMeta, ModelArgs, ModelMeta};

pub fn entry_point(input: syn::DeriveInput) -> darling::Result<TokenStream> {
    let mut acc = Error::accumulator();

    let model_meta = acc
        .handle(ModelMeta::from_derive_input(&input))
        .unwrap_or_default();

    let Some(fields) = acc.handle(find_named_fields(&input.data)) else {
        return finish_as_error(acc);
    };

    let mut parsed_fields = Vec::new();

    for pair in fields.named.pairs() {
        // exclude non-serialized fields from the output
        let field = pair.into_value();
        if let Some(args) = acc.handle(FieldMeta::from_attributes(&field.attrs))
            && let Some(serde) = acc.handle(FieldSerdeMeta::from_attributes(&field.attrs))
            && !serde.has_skip()
        {
            let ident = field.ident.as_ref().expect("must be named fields here");
            parsed_fields.push(FieldArgs {
                name: ident,
                ty: &field.ty,
                args,
                serde,
            });
        }
    }

    if model_meta.fields_only.is_present() {
        for field in &mut parsed_fields {
            field.args.filter = false;
            field.args.partial = false;
        }
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
        internals_name: format_ident!("__{}ModelDocumentInternals", input.ident),
        fields: parsed_fields,
        crate_: model_meta
            .crate_
            .unwrap_or_else(|| syn::parse_quote!(::bson_model)),
        derive_partial: model_meta
            .derive_partial
            .as_deref()
            .map_or(&default_derive, Vec::as_slice),
        derive_filter: model_meta
            .derive_filter
            .as_deref()
            .map_or(&default_derive, Vec::as_slice),
    };

    let mut output = emit_internals(&args);
    output.extend(emit_partial(&args));
    output.extend(emit_filter(&args));
    output.extend(emit_sort(&args));
    output.extend(emit_fields(&args));

    if let Err(err) = acc.finish() {
        output.extend(err.write_errors());
    }

    Ok(output)
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

    let field_methods = fields.iter()
        .filter(|field| field.serde.has_with())
        .map(|field| {
            let FieldArgs { name, ty, serde, args } = field;
            let FieldSerdeMeta { with, serialize_with, .. } = serde;

            let partial_with_name = format_ident!("partial_{}", name);
            let filter_with_name = format_ident!("filter_{}", name);
            let source_with = with
                .as_ref()
                .map(|w| quote::quote! { #w::serialize })
                .unwrap_or_else(|| {
                    let w = serialize_with.as_ref().expect("must be specified");
                    quote::quote! { #w }
                });

            let mut part = TokenStream::new();

            if args.partial {
                part.extend(quote::quote! {
                    fn #partial_with_name<S>(field: &::std::option::Option<#ty>, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                    where
                        S: #crate_::private::serde::ser::Serializer,
                    {
                        match field {
                            ::std::option::Option::Some(value) => #source_with(value, serializer),
                            ::std::option::Option::None => serializer.serialize_none(),
                        }
                    }
                });
            }

            if args.filter {
                part.extend(quote::quote! {
                    fn #filter_with_name<S>(field: &::std::option::Option<#crate_::Filter<#ty>>, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                    where
                        S: #crate_::private::serde::ser::Serializer,
                    {
                        struct __With #impl_gen (#crate_::private::Never<#ty_name #ty_gen>) #where_clause;

                        impl #impl_gen #crate_::private::serde_with::SerializeAs<#ty> for __With #ty_gen #where_clause {
                            fn serialize_as<S>(source: &#ty, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                            where
                                S: #crate_::private::serde::Serializer,
                            {
                                #source_with(source, serializer)
                            }
                        }

                        match field {
                            ::std::option::Option::Some(value) => #crate_::private::serde_with::As::<
                                #crate_::Filter<__With #ty_gen>
                            >::serialize(value, serializer),
                            ::std::option::Option::None => serializer.serialize_none(),
                        }
                    }
                });
            }

            part
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
        #[allow(dead_code)]
        struct #internals_name #impl_gen (#crate_::private::Never<#ty_name #ty_gen>) #where_clause;

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

    let fields = fields.iter().filter(|f| f.args.partial);

    let field_decls = fields.clone().map(|field| {
        let FieldArgs {
            name, ty, serde, ..
        } = field;

        let with = if serde.has_with() {
            let ident = format_ident!("partial_{}", name);
            Some(format!("{internals_name}{turbo_fish}::{ident}"))
        } else {
            None
        }
        .into_iter();

        let rename = serde.rename.as_ref().map(ToString::to_string).into_iter();

        quote::quote! {
            #(#[serde(serialize_with = #with)])*
            #(#[serde(rename = #rename)])*
            #[serde(skip_serializing_if = "::std::option::Option::is_none")]
            pub #name: ::std::option::Option<#ty>,
        }
    });

    let field_methods = fields.clone().map(|field| {
        let FieldArgs { name, ty, .. } = field;
        let doc = format!("Sets the `{name}` field.");

        quote::quote! {
            #[doc = #doc]
            #[must_use]
            pub fn #name(mut self, #name: #ty) -> Self {
                self.#name = ::std::option::Option::Some(#name);
                self
            }
        }
    });

    let field_defaults = fields.map(|field| {
        let FieldArgs { name, .. } = field;
        quote::quote! {
            #name: None,
        }
    });

    let into_document = emit_into_document(crate_, partial_name, generics);
    let serde_crate = quote::quote!(#crate_::private::serde).to_string();

    let doc = format!("A partial [`{ty_name}`].");
    quote::quote! {
        #[doc = #doc]
        #[derive(#crate_::private::serde::Serialize #(,#derive_partial)*)]
        #[serde(crate = #serde_crate)]
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

    let fields = fields.iter().filter(|f| f.args.filter);

    let field_decls = fields.clone().map(|field| {
        let FieldArgs {
            name, ty, serde, ..
        } = field;
        let with = if serde.has_with() {
            let ident = format_ident!("filter_{}", name);
            Some(format!("{internals_name}{turbo_fish}::{ident}"))
        } else {
            None
        }
        .into_iter();

        let rename = serde.rename.as_ref().map(ToString::to_string).into_iter();

        quote::quote! {
            #(#[serde(serialize_with = #with)])*
            #(#[serde(rename = #rename)])*
            #[serde(skip_serializing_if = "::std::option::Option::is_none")]
            pub #name: ::std::option::Option<#crate_::Filter<#ty>>,
        }
    });

    let field_methods = fields.clone().map(|field| {
        let FieldArgs { name, ty, .. } = field;
        let doc = format!("Sets the filter condition for the `{name}` field.");

        quote::quote! {
            #[doc = #doc]
            #[must_use]
            pub fn #name(mut self, #name: impl ::std::convert::Into<#crate_::Filter<#ty>>) -> Self {
                self.#name = Some(::std::convert::Into::into(#name));
                self
            }
        }
    });

    let field_defaults = fields.map(|field| {
        let FieldArgs { name, .. } = field;
        quote::quote! {
            #name: None,
        }
    });

    let into_document = emit_into_document(crate_, filter_name, generics);
    let serde_crate = quote::quote!(#crate_::private::serde).to_string();

    let doc = format!("A filter builder for [`{ty_name}`].");
    quote::quote! {
        #[doc = #doc]
        #[derive(#crate_::private::serde::Serialize #(, #derive_filter)*)]
        #[serde(crate = #serde_crate)]
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

    let fields = fields.iter().filter(|f| f.args.filter);

    let field_methods = fields.map(|field| {
        let FieldArgs { name, serde, .. } = field;
        let rename = serde.rename.as_ref().unwrap_or(name).to_string();
        let doc = format!("Sorts the document by the `{name}` field.");

        quote::quote! {
            #[doc = #doc]
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

    let doc_header = format!("A sort builder for [`{ty_name}`].");
    let doc_footer = format!(
        "[`Document`]: {}::private::bson::Document",
        crate_
            .to_token_stream()
            .to_string()
            .split_whitespace()
            .collect::<String>()
    );
    quote::quote! {
        #[doc = #doc_header]
        ///
        /// This represents a thin wrapper around a [`Document`] to retain the used sort priority.
        ///
        #[doc = #doc_footer]
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
        ty_name,
        fields_name,
        fields,
        crate_,
        ..
    } = args;

    let field_methods = fields.iter().map(|field| {
        let FieldArgs { name, serde, .. } = field;
        let rename = serde.rename.as_ref().unwrap_or(name).to_string();
        let expr_name = "$".to_owned() + rename.strip_prefix("r#").unwrap_or(&rename);
        let doc = format!("Gets the BSON `{name}` field.");

        quote::quote! {
            #[doc = #doc]
            pub const fn #name(self) -> #crate_::ModelField {
                const {
                    #crate_::ModelField::new(#expr_name)
                }
            }
        }
    });

    let doc = format!("Accessor struct for the BSON fields of [`{ty_name}`].");
    quote::quote! {
        #[doc = #doc]
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

    for pair in generics.params.pairs() {
        if let GenericParam::Type(ty) = pair.into_value() {
            let ident = &ty.ident;
            where_clause
                .predicates
                .push(syn::parse_quote!(#ident: #crate_::private::serde::Serialize));
        }
    }

    quote::quote! {
        impl #impl_gen #ty_name #ty_gen #where_clause {
            /// Tries to serialize this value into a BSON document.
            ///
            /// # Errors
            ///
            /// Returns `Err` if the update could not be serialized as a `Document`.
            /// This could imply that `Self` is not compatible with BSON serialization.
            pub fn into_document(self) -> #crate_::private::bson::error::Result<#crate_::private::bson::Document> {
                #crate_::private::bson::serialize_to_document(&self)
            }
        }

        #[automatically_derived]
        impl #impl_gen TryFrom<#ty_name #ty_gen> for #crate_::private::bson::Document #where_clause {
            type Error = #crate_::private::bson::error::Error;

            fn try_from(value: #ty_name #ty_gen) -> Result<Self, Self::Error> {
                value.into_document()
            }
        }
    }
}

fn find_named_fields(data: &Data) -> darling::Result<&FieldsNamed> {
    if let Data::Struct(data) = data
        && let Fields::Named(raw_fields) = &data.fields
    {
        return Ok(raw_fields);
    }

    // just keep the call site span (i.e. the derive itself)
    Err(Error::custom(
        "`ModelDocument` can only be derived on struct with named fields",
    ))
}

pub fn finish_as_error<T>(acc: darling::error::Accumulator) -> darling::Result<T> {
    Err(Error::multiple(acc.into_inner()))
}
