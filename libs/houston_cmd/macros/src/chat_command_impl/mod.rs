use darling::{Error, FromMeta as _};
use proc_macro2::TokenStream;
use syn::Item;

use crate::any_command_impl::{to_command_option_shared, to_command_shared};
use crate::args::{ChatCommandArgs, TopSubCommandArgs};

mod command_emit;
mod group_emit;

use command_emit::to_command_option_command;
use group_emit::to_command_option_group;

pub fn entry_point(args: TokenStream, item: TokenStream) -> darling::Result<TokenStream> {
    let args = darling::ast::NestedMeta::parse_meta_list(args)?;
    let args = ChatCommandArgs::from_list(&args)?;

    match syn::parse2(item)? {
        Item::Fn(mut func) => {
            let command_option = to_command_option_command(&mut func, args.name, &args.main.common);
            Ok(to_command_shared(
                &func.vis,
                &func.sig.ident,
                command_option,
                args.main,
            ))
        },
        Item::Mod(mut module) => {
            let command_option =
                to_command_option_group(&mut module, args.name, &args.main.common)?;
            Ok(to_command_shared(
                &module.vis,
                &module.ident,
                command_option,
                args.main,
            ))
        },
        item => Err(Error::custom("expected an `fn` or `mod` item").with_span(&item)),
    }
}

pub fn sub_entry_point(args: TokenStream, item: TokenStream) -> darling::Result<TokenStream> {
    let args = darling::ast::NestedMeta::parse_meta_list(args)?;
    let args = TopSubCommandArgs::from_list(&args)?;

    match syn::parse2(item)? {
        Item::Fn(mut func) => {
            let command_option = to_command_option_command(&mut func, args.name, &args.common);
            Ok(to_command_option_shared(
                &func.vis,
                &func.sig.ident,
                command_option,
                &args.common,
            ))
        },
        Item::Mod(mut module) => {
            let command_option = to_command_option_group(&mut module, args.name, &args.common)?;
            Ok(to_command_option_shared(
                &module.vis,
                &module.ident,
                command_option,
                &args.common,
            ))
        },
        item => Err(Error::custom("expected an `fn` or `mod` item").with_span(&item)),
    }
}
