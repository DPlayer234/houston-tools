use proc_macro2::{Span, TokenStream};
use syn::{Ident, Visibility};

use crate::args::AnyCommandArgs;
use crate::util::{quote_map_option, warning};

pub fn to_command_shared(
    vis: &Visibility,
    ident: &Ident,
    command_option: TokenStream,
    args: AnyCommandArgs,
) -> syn::Result<TokenStream> {
    let warning = (args.contexts.is_none() || args.integration_types.is_none()).then(|| {
        warning(
            Span::call_site(),
            "specify both `contexts` and `integration_types`, discord's defaults can be faulty",
        )
    });

    let contexts = quote_map_option(args.contexts, |c| {
        let c = c.into_iter();
        quote::quote! {
            ::std::borrow::Cow::Borrowed(&[
                #( ::houston_cmd::private::serenity::InteractionContext:: #c, )*
            ])
        }
    });

    let integration_types = quote_map_option(args.integration_types, |c| {
        let c = c.into_iter();
        quote::quote! {
            ::std::borrow::Cow::Borrowed(&[
                #( ::houston_cmd::private::serenity::InstallationContext:: #c, )*
            ])
        }
    });

    let permissions = quote_map_option(args.default_member_permissions, |c| {
        let mut c = c.into_iter();
        if let Some(first) = c.next() {
            quote::quote! {
                ::houston_cmd::private::serenity::Permissions:: #first #( .union(::houston_cmd::private::serenity::Permissions:: #c) )*
            }
        } else {
            quote::quote! { ::houston_cmd::private::serenity::Permissions::empty() }
        }
    });

    let nsfw = args.nsfw;

    Ok(quote::quote! {
        #warning
        #vis const fn #ident() -> ::houston_cmd::model::Command {
            const {
                ::houston_cmd::model::Command {
                    contexts: #contexts,
                    integration_types: #integration_types,
                    default_member_permissions: #permissions,
                    nsfw: #nsfw,
                    data: #command_option,
                }
            }
        }
    })
}

pub fn to_command_option_shared(
    vis: &Visibility,
    ident: &Ident,
    command_option: TokenStream,
) -> syn::Result<TokenStream> {
    Ok(quote::quote! {
        #vis const fn #ident() -> ::houston_cmd::model::CommandOption {
            const {
                #command_option
            }
        }
    })
}
