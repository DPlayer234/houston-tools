use std::mem::take;

use darling::FromMeta as _;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::ext::IdentExt as _;
use syn::spanned::Spanned as _;
use syn::{Attribute, Item, ItemMod, ItemUse, Meta, UseTree, Visibility};

use super::command_emit::to_command_option_command;
use crate::args::SubCommandArgs;
use crate::util::{ensure_span, ensure_spanned, extract_description, warning};

pub fn to_command_option_group(
    module: &mut ItemMod,
    name: Option<String>,
) -> syn::Result<TokenStream> {
    let content = match module.content.as_mut() {
        Some(content) => &mut content.1,
        None => return Err(syn::Error::new_spanned(module, "command must have a body")),
    };

    let mut other_items = Vec::new();
    let mut sub_commands = Vec::new();
    let mut warnings = Vec::new();

    for item in take(content) {
        if let Some(vis) = item_vis(&item) {
            if *vis != Visibility::Inherited {
                warnings.push(warning(
                    vis.span(),
                    "remove this `pub`, visibility has no effect within a command group",
                ));
            }
        }

        match item {
            Item::Fn(mut item) => {
                if let Some(attr) = find_sub_command_attr(&mut item.attrs) {
                    let args = parse_sub_command_args(&attr.meta)?;
                    let tokens = to_command_option_command(&mut item, args.name)?;
                    sub_commands.push(tokens);
                } else {
                    other_items.push(Item::Fn(item))
                }
            },
            Item::Mod(mut item) => {
                if let Some(attr) = find_sub_command_attr(&mut item.attrs) {
                    let args = parse_sub_command_args(&attr.meta)?;
                    let tokens = to_command_option_group(&mut item, args.name)?;
                    sub_commands.push(tokens);
                } else {
                    other_items.push(Item::Mod(item))
                }
            },
            Item::Use(mut item) => {
                if let Some(paths) = use_include(&mut item)? {
                    sub_commands.extend(paths.into_iter().map(|t| quote::quote! { #t() }));
                } else {
                    other_items.push(Item::Use(item));
                }
            },
            item => other_items.push(item),
        }
    }

    if sub_commands.is_empty() {
        return Err(syn::Error::new_spanned(
            module,
            "command group must have at least one #[sub_command] `fn`, `mod`, or `use`",
        ));
    }

    let name = name.unwrap_or_else(|| module.ident.unraw().to_string());
    let description = extract_description(&module.attrs).ok_or_else(|| {
        syn::Error::new_spanned(&module, "a description is required, add a doc comment")
    })?;

    ensure_spanned!(&module.ident, (1..=32).contains(&name.chars().count()) => "the name must be 1 to 32 characters long");
    ensure_span!(description.span(), (1..=100).contains(&description.chars().count()) => "the description must be 1 to 100 characters long");
    ensure_spanned!(module, (1..=25).contains(&sub_commands.len()) => "there must be 1 to 25 sub commands");

    let description = &*description;
    Ok(quote::quote! {{
        #(#warnings)*
        #(#other_items)*

        ::houston_cmd::model::CommandOption {
            name: ::std::borrow::Cow::Borrowed(#name),
            description: ::std::borrow::Cow::Borrowed(#description),
            data: ::houston_cmd::model::CommandOptionData::Group(::houston_cmd::model::GroupData {
                // this const-block is necessary to satisfy the compiler when the list
                // involves function calls in place of a sub-command struct literal
                sub_commands: ::std::borrow::Cow::Borrowed(const { &[
                    #(#sub_commands),*
                ] }),
            }),
        }
    }})
}

fn find_sub_command_attr(vec: &mut Vec<Attribute>) -> Option<Attribute> {
    find_and_take(vec, |a| a.meta.path().is_ident("sub_command"))
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

fn use_include(item: &mut ItemUse) -> syn::Result<Option<Vec<TokenStream>>> {
    let Some(attr) = find_sub_command_attr(&mut item.attrs) else {
        return Ok(None);
    };

    ensure_spanned!(
        attr,
        matches!(attr.meta, Meta::Path(_)) =>
        "`#[sub_command] use` cannot specify additional parameters"
    );

    fn resolve_tree(
        buf: &mut Vec<TokenStream>,
        prefix: Option<&dyn ToTokens>,
        tree: &UseTree,
    ) -> syn::Result<()> {
        match tree {
            UseTree::Path(path) => {
                let ident = &path.ident;
                let colon = &path.colon2_token;
                let prefix = quote::quote!(#prefix #ident #colon);
                resolve_tree(buf, Some(&prefix), &path.tree)
            },
            UseTree::Name(name) => {
                let ident = &name.ident;
                buf.push(quote::quote!(#prefix #ident));
                Ok(())
            },
            UseTree::Rename(_) => Err(syn::Error::new_spanned(
                tree,
                "cannot rename `#[sub_command] use`",
            )),
            UseTree::Glob(_) => Err(syn::Error::new_spanned(
                tree,
                "cannot glob-import `#[sub_command] use`",
            )),
            UseTree::Group(group) => {
                for item in &group.items {
                    resolve_tree(buf, prefix, item)?;
                }
                Ok(())
            },
            #[allow(unreachable_patterns)]
            _ => Err(syn::Error::new_spanned(
                tree,
                "unknown `#[sub_command] use` pattern",
            )),
        }
    }

    let mut buf = Vec::new();

    let prefix = item
        .leading_colon
        .as_ref()
        .map(|colon| colon as &dyn ToTokens);

    resolve_tree(&mut buf, prefix, &item.tree)?;
    Ok(Some(buf))
}

fn item_vis(item: &Item) -> Option<&Visibility> {
    match item {
        Item::Const(item) => Some(&item.vis),
        Item::Enum(item) => Some(&item.vis),
        Item::ExternCrate(item) => Some(&item.vis),
        Item::Fn(item) => Some(&item.vis),
        Item::ForeignMod(_) => None,
        Item::Impl(_) => None,
        Item::Macro(_) => None,
        Item::Mod(item) => Some(&item.vis),
        Item::Static(item) => Some(&item.vis),
        Item::Struct(item) => Some(&item.vis),
        Item::Trait(item) => Some(&item.vis),
        Item::TraitAlias(item) => Some(&item.vis),
        Item::Type(item) => Some(&item.vis),
        Item::Union(item) => Some(&item.vis),
        Item::Use(item) => Some(&item.vis),
        _ => None,
    }
}
