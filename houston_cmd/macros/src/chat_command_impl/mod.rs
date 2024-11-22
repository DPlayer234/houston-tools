use darling::FromMeta;
use proc_macro2::TokenStream;
use syn::spanned::Spanned;
use syn::{ItemFn, ItemMod};

mod command_emit;
mod group_emit;

use command_emit::to_command_option_command;
use group_emit::to_command_option_group;

use crate::any_command_impl::to_command_shared;
use crate::args::ChatCommandArgs;

pub fn entry_point(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args = darling::ast::NestedMeta::parse_meta_list(args)?;
    let args = ChatCommandArgs::from_list(&args)?;

    if let Ok(module) = syn::parse2(item.clone()) {
        to_command_group(module, args)
    } else {
        let func = syn::parse2(item)?;
        to_command(func, args)
    }
}

fn to_command(mut func: ItemFn, args: ChatCommandArgs) -> syn::Result<TokenStream> {
    if func.sig.asyncness.is_none() {
        return Err(syn::Error::new(func.sig.span(), "command function must be async"));
    }

    let command_option = to_command_option_command(&mut func, args.name)?;
    to_command_shared(&func.vis, &func.sig.ident, command_option, args.main)
}

fn to_command_group(mut module: ItemMod, args: ChatCommandArgs) -> syn::Result<TokenStream> {
    let command_option = to_command_option_group(&mut module, args.name)?;
    to_command_shared(&module.vis, &module.ident, command_option, args.main)
}
