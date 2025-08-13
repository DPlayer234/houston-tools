use proc_macro2::{Span, TokenStream};
use syn::{Ident, Visibility};

use crate::args::{AnyCommandArgs, CommonArgs};
use crate::util::{quote_map_option, warning};

pub fn to_command_shared(
    vis: &Visibility,
    ident: &Ident,
    command_option: TokenStream,
    args: AnyCommandArgs,
) -> TokenStream {
    let CommonArgs { crate_ } = &args.common;

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
                #( #crate_::private::serenity::InteractionContext:: #c, )*
            ])
        }
    });

    let integration_types = quote_map_option(args.integration_types, |c| {
        let c = c.into_iter();
        quote::quote! {
            ::std::borrow::Cow::Borrowed(&[
                #( #crate_::private::serenity::InstallationContext:: #c, )*
            ])
        }
    });

    let permissions = quote_map_option(args.default_member_permissions, |c| {
        let mut c = c.into_iter();
        if let Some(first) = c.next() {
            quote::quote! {
                #crate_::private::serenity::Permissions:: #first #( .union(#crate_::private::serenity::Permissions:: #c) )*
            }
        } else {
            quote::quote! { #crate_::private::serenity::Permissions::empty() }
        }
    });

    let nsfw = args.nsfw;

    quote::quote! {
        #warning
        #vis const fn #ident() -> #crate_::model::Command {
            const {
                #crate_::model::Command::builder()
                    .contexts(#contexts)
                    .integration_types(#integration_types)
                    .default_member_permissions(#permissions)
                    .nsfw(#nsfw)
                    .data(#command_option)
                    .build()
            }
        }
    }
}

pub fn to_command_option_shared(
    vis: &Visibility,
    ident: &Ident,
    command_option: TokenStream,
    args: &CommonArgs,
) -> TokenStream {
    let CommonArgs { crate_ } = args;

    quote::quote! {
        #vis const fn #ident() -> #crate_::model::CommandOption {
            const {
                #command_option
            }
        }
    }
}
