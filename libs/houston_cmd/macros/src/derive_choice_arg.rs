use darling::ast::NestedMeta;
use darling::{Error, FromDeriveInput as _, FromMeta as _};
use proc_macro2::TokenStream;
use syn::ext::IdentExt as _;
use syn::{Data, Fields};

use crate::args::{ChoiceArgArgs, ChoiceArgVariantArgs};

pub fn entry_point(input: syn::DeriveInput) -> darling::Result<TokenStream> {
    let args = ChoiceArgArgs::from_derive_input(&input)?;
    let crate_ = args.common.crate_;

    let Data::Enum(data) = input.data else {
        let err = Error::custom("choice args must be enums");
        return Err(err.with_span(&input));
    };

    let mut acc = Error::accumulator();

    let mut names = Vec::new();
    let mut idents = Vec::new();

    for variant in data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            let err = Error::custom("choice arg variants cannot have fields");
            acc.push(err.with_span(&variant));
            continue;
        }

        let attrs: Vec<_> = variant
            .attrs
            .into_iter()
            .map(|attr| NestedMeta::Meta(attr.meta))
            .collect();

        if let Some(attrs) = acc.handle(ChoiceArgVariantArgs::from_list(&attrs)) {
            let name = attrs
                .name
                .unwrap_or_else(|| variant.ident.unraw().to_string());

            if !(1..=100).contains(&name.chars().count()) {
                let err = Error::custom("the name must be 1 to 100 characters long");
                acc.push(err.with_span(&variant.ident));
            }

            names.push(name);
            idents.push(variant.ident);
        }
    }

    let enum_ident = &input.ident;
    let indices = 0..idents.len();

    let errors = acc.finish().err().map(|e| e.write_errors());
    Ok(quote::quote! {
        #[automatically_derived]
        impl #crate_::ChoiceArg for #enum_ident {
            fn list() -> ::std::borrow::Cow<'static, [#crate_::model::Choice]> {
                ::std::borrow::Cow::Borrowed(const { &[
                    #(
                        #crate_::model::Choice::builder()
                            .name(::std::borrow::Cow::Borrowed(#names))
                            .build(),
                    )*
                ] })
            }

            fn from_index(index: usize) -> Option<Self> {
                match index {
                    #(
                        #indices => ::std::option::Option::Some(#enum_ident::#idents),
                    )*
                    _ => ::std::option::Option::None,
                }
            }
        }

        #errors
    })
}
