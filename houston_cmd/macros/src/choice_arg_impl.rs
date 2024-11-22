use darling::ast::NestedMeta;
use darling::FromMeta;
use proc_macro2::TokenStream;
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::{Data, Fields};

#[derive(Debug, darling::FromMeta)]
struct VariantArgs {
    name: Option<String>,
}

pub fn entry_point(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let Data::Enum(data) = input.data else {
        return Err(syn::Error::new(input.span(), "choice args must be enums"));
    };

    let mut names = Vec::new();
    let mut idents = Vec::new();

    for variant in data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new(variant.span(), "choice arg variants cannot have fields"));
        }

        let attrs: Vec<_> = variant.attrs
            .into_iter()
            .map(|attr| NestedMeta::Meta(attr.meta))
            .collect();
        let attrs = VariantArgs::from_list(&attrs)?;

        names.push(attrs.name.unwrap_or_else(|| variant.ident.unraw().to_string()));
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
