use proc_macro2::TokenStream;
use syn::fold::Fold;
use syn::{Attribute, Expr, ExprLit, Lit, Meta, MetaNameValue};

pub fn quote_map_option<T>(value: Option<T>, f: impl FnOnce(T) -> TokenStream) -> TokenStream {
    match value {
        Some(value) => {
            let value = f(value);
            quote::quote! { ::std::option::Option::Some(#value) }
        },
        None => quote::quote! { ::std::option::Option::None },
    }
}

pub fn extract_description(attrs: &[Attribute]) -> Option<String> {
    let ident = quote::format_ident!("doc");

    let mut desc = String::new();
    for a in attrs {
        if let Meta::NameValue(MetaNameValue {
            path,
            value:
                Expr::Lit(ExprLit {
                    lit: Lit::Str(literal),
                    ..
                }),
            ..
        }) = &a.meta
        {
            if path.is_ident(&ident) {
                if !desc.is_empty() {
                    desc.push(' ');
                }

                desc.push_str(literal.value().trim());
            }
        }
    }

    (!desc.is_empty()).then_some(desc)
}

pub struct FoldLifetimeAsStatic;
impl Fold for FoldLifetimeAsStatic {
    fn fold_lifetime(&mut self, _i: syn::Lifetime) -> syn::Lifetime {
        syn::parse_quote! { 'static }
    }
}

macro_rules! ensure_spanned {
    ($span:expr, $cond:expr => $($t:tt)*) => {
        if !$cond {
            return Err(syn::Error::new_spanned($span, format_args!($($t)*)))
        }
    };
}

pub(crate) use ensure_spanned;
