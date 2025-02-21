use darling::FromMeta;
use darling::ast::NestedMeta;
use proc_macro2::TokenStream;
use syn::ext::IdentExt;
use syn::{Data, Fields};

use crate::util::ensure_spanned;

#[derive(Debug, darling::FromMeta)]
struct VariantArgs {
    name: Option<String>,
}

pub fn entry_point(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let Data::Enum(data) = input.data else {
        return Err(syn::Error::new_spanned(input, "choice args must be enums"));
    };

    let mut names = Vec::new();
    let mut idents = Vec::new();

    for variant in data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                variant,
                "choice arg variants cannot have fields",
            ));
        }

        let attrs: Vec<_> = variant
            .attrs
            .into_iter()
            .map(|attr| NestedMeta::Meta(attr.meta))
            .collect();
        let attrs = VariantArgs::from_list(&attrs)?;

        let name = attrs
            .name
            .unwrap_or_else(|| variant.ident.unraw().to_string());
        ensure_spanned!(variant.ident, (1..=100).contains(&name.chars().count()) => "the name must be 1 to 100 characters long");

        names.push(name);
        idents.push(variant.ident);
    }

    let enum_ident = &input.ident;
    let indices = 0..idents.len();

    Ok(quote::quote! {
        #[automatically_derived]
        impl ::houston_cmd::ChoiceArg for #enum_ident {
            fn list() -> ::std::borrow::Cow<'static, [::houston_cmd::model::Choice]> {
                ::std::borrow::Cow::Borrowed(&[
                    #(
                        ::houston_cmd::model::Choice {
                            name: ::std::borrow::Cow::Borrowed(#names)
                        },
                    )*
                ])
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
    })
}
