use darling::FromMeta as _;
use proc_macro2::TokenStream;
use syn::{Item, ItemFn, ItemMod};

use crate::any_command_impl::{to_command_option_shared, to_command_shared};
use crate::args::{ChatCommandArgs, TopSubCommandArgs};

mod command_emit;
mod group_emit;

use command_emit::to_command_option_command;
use group_emit::to_command_option_group;

pub fn entry_point(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args = darling::ast::NestedMeta::parse_meta_list(args)?;
    let args = ChatCommandArgs::from_list(&args)?;

    match syn::parse2(item)? {
        Item::Fn(mut func) => {
            let command_option =
                to_command_option_command(&mut func, args.name, &args.main.common)?;
            to_command_shared(&func.vis, &func.sig.ident, command_option, args.main)
        },
        Item::Mod(mut module) => {
            let command_option =
                to_command_option_group(&mut module, args.name, &args.main.common)?;
            to_command_shared(&module.vis, &module.ident, command_option, args.main)
        },
        item => Err(syn::Error::new_spanned(
            item,
            "expected an `fn` or `mod` item",
        )),
    }
}

pub fn sub_entry_point(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args = darling::ast::NestedMeta::parse_meta_list(args)?;
    let args = TopSubCommandArgs::from_list(&args)?;

    if let Ok(mut module) = syn::parse2::<ItemMod>(item.clone()) {
        let command_option = to_command_option_group(&mut module, args.name, &args.common)?;
        to_command_option_shared(&module.vis, &module.ident, command_option, &args.common)
    } else {
        let mut func = syn::parse2::<ItemFn>(item)?;
        let command_option = to_command_option_command(&mut func, args.name, &args.common)?;
        to_command_option_shared(&func.vis, &func.sig.ident, command_option, &args.common)
    }
}
