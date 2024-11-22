use std::mem::take;

use darling::FromMeta;
use proc_macro2::TokenStream;
use syn::ext::IdentExt;
use syn::{Attribute, Item, ItemMod, Meta};

use super::command_emit::to_command_option_command;

use crate::args::SubCommandArgs;
use crate::util::{ensure_spanned, extract_description};

pub fn to_command_option_group(module: &mut ItemMod, name: Option<String>) -> syn::Result<TokenStream> {
    let content_src = match module.content.as_mut() {
        Some(content) => &mut content.1,
        None => return Err(syn::Error::new_spanned(module, "command must have a body")),
    };

    let content = take(content_src);
    let mut use_items = Vec::new();
    let mut sub_commands = Vec::new();

    for item in content {
        match item {
            Item::Fn(mut item) => if let Some(attr) = find_sub_command_attr(&mut item.attrs) {
                let args = parse_sub_command_args(&attr.meta)?;
                let tokens = to_command_option_command(&mut item, args.name)?;
                sub_commands.push(tokens);
            } else {
                return Err(syn::Error::new_spanned(item, "function must be attributed with #[sub_command]"));
            },
            Item::Mod(mut item) => if let Some(attr) = find_sub_command_attr(&mut item.attrs) {
                let args = parse_sub_command_args(&attr.meta)?;
                let tokens = to_command_option_group(&mut item, args.name)?;
                sub_commands.push(tokens);
            } else {
                return Err(syn::Error::new_spanned(item, "group must be attributed with #[sub_command]"));
            },
            Item::Use(item) => use_items.push(item),
            _ => return Err(syn::Error::new_spanned(item, "only `use`, `fn`, and `mod` items are allowed in a #[chat_command]")),
        }
    }

    if sub_commands.is_empty() {
        return Err(syn::Error::new_spanned(module, "command group must have at least one #[sub_command] function"));
    }

    let name = name.unwrap_or_else(|| module.ident.unraw().to_string());
    let description = extract_description(&module.attrs)
        .ok_or_else(|| syn::Error::new_spanned(&module, "a description is required, add a doc comment"))?;

    ensure_spanned!(module, (1..=32).contains(&name.chars().count()) => "the name must be 1 to 32 characters long");
    ensure_spanned!(module, (1..=100).contains(&description.chars().count()) => "the description must be 1 to 100 characters long");
    ensure_spanned!(module, (1..=25).contains(&sub_commands.len()) => "there must be 1 to 25 sub commands");

    Ok(quote::quote! {{
        #(#use_items)*

        ::houston_cmd::model::CommandOption {
            name: ::std::borrow::Cow::Borrowed(#name),
            description: ::std::borrow::Cow::Borrowed(#description),
            data: ::houston_cmd::model::CommandOptionData::Group(::houston_cmd::model::GroupData {
                sub_commands: ::std::borrow::Cow::Borrowed(&[
                    #(#sub_commands),*
                ]),
            }),
        }
    }})
}

fn find_sub_command_attr(vec: &mut Vec<Attribute>) -> Option<Attribute> {
    let ident = quote::format_ident!("sub_command");
    find_and_take(vec, |a| match &a.meta {
        Meta::Path(path) => path.is_ident(&ident),
        Meta::List(meta_list) => meta_list.path.is_ident(&ident),
        _ => false,
    })
}

fn find_and_take<T>(vec: &mut Vec<T>, mut f: impl FnMut(&T) -> bool) -> Option<T> {
    let index = vec.iter().enumerate().find(move |(_, item)| f(item))?.0;
    Some(vec.remove(index))
}

fn parse_sub_command_args(args: &Meta) -> darling::Result<SubCommandArgs> {
    match args {
        Meta::Path(_) => Ok(SubCommandArgs::default()),
        _ => SubCommandArgs::from_meta(args),
    }
}
