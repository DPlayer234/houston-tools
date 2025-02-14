use darling::util::SpannedValue;
use proc_macro2::{Span, TokenStream};
use syn::fold::Fold;
use syn::spanned::Spanned;
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

pub fn extract_description(attrs: &[Attribute]) -> Option<SpannedValue<String>> {
    let ident = quote::format_ident!("doc");

    let mut res = None;
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
                let desc = res.get_or_insert(SpannedValue::new(String::new(), a.span()));
                if !desc.is_empty() {
                    desc.push(' ');
                }

                desc.push_str(literal.value().trim());
            }
        }
    }

    res
}

pub struct ReplaceLifetimes {
    l: Option<syn::Lifetime>,
}

impl ReplaceLifetimes {
    pub fn omit() -> Self {
        Self { l: None }
    }

    #[expect(dead_code, reason = "maybe useful later")]
    pub fn new(l: &str) -> Self {
        Self {
            l: Some(syn::Lifetime::new(l, Span::call_site())),
        }
    }

    fn resolve(&self, i: Option<syn::Lifetime>) -> Option<syn::Lifetime> {
        if i.as_ref().is_none_or(|i| i.ident != "static") {
            self.l.clone()
        } else {
            i
        }
    }
}

impl Fold for ReplaceLifetimes {
    fn fold_type_reference(&mut self, mut i: syn::TypeReference) -> syn::TypeReference {
        i.lifetime = self.resolve(i.lifetime);
        syn::fold::fold_type_reference(self, i)
    }

    fn fold_lifetime(&mut self, i: syn::Lifetime) -> syn::Lifetime {
        self.resolve(Some(i))
            .unwrap_or_else(|| syn::Lifetime::new("'_", Span::call_site()))
    }
}

macro_rules! ensure_span {
    ($span:expr, $cond:expr => $($t:tt)*) => {
        if !$cond {
            return Err(syn::Error::new($span, format_args!($($t)*)))
        }
    };
}

macro_rules! ensure_spanned {
    ($span:expr, $cond:expr => $($t:tt)*) => {
        if !$cond {
            return Err(syn::Error::new_spanned($span, format_args!($($t)*)))
        }
    };
}

pub(crate) use {ensure_span, ensure_spanned};
