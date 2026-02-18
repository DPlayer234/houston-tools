use std::mem::take;

use darling::{Error, FromMeta as _};
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::ext::IdentExt as _;
use syn::spanned::Spanned as _;
use syn::{Attribute, Item, ItemMod, ItemUse, Meta, UseTree, Visibility};

use super::command_emit::to_command_option_command;
use crate::args::{CommonArgs, SubCommandArgs};
use crate::util::{extract_description, warning};

pub fn to_command_option_group(
    module: &mut ItemMod,
    name: Option<String>,
    args: &CommonArgs,
) -> darling::Result<TokenStream> {
    let content = match module.content.as_mut() {
        Some(content) => &mut content.1,
        None => return Err(Error::custom("command must have a body").with_span(&module)),
    };

    let mut acc = Error::accumulator();

    let mut other_items = Vec::new();
    let mut sub_commands = Vec::new();
    let mut warnings = Vec::new();

    for item in take(content) {
        if let Some(vis) = item_vis(&item)
            && *vis != Visibility::Inherited
        {
            warnings.push(warning(
                vis.span(),
                "remove this `pub`, visibility has no effect within a command group",
            ));
        }

        match item {
            Item::Fn(mut item) => {
                if let Some(attr) = find_sub_command_attr(&mut item.attrs)
                    && let Some(sub_args) = acc.handle(parse_sub_command_args(&attr.meta))
                {
                    let tokens = to_command_option_command(&mut item, sub_args.name, args);
                    sub_commands.push(tokens);
                } else {
                    other_items.push(Item::Fn(item))
                }
            },
            Item::Mod(mut item) => {
                if let Some(attr) = find_sub_command_attr(&mut item.attrs)
                    && let Some(sub_args) = acc.handle(parse_sub_command_args(&attr.meta))
                    && let Some(tokens) =
                        acc.handle(to_command_option_group(&mut item, sub_args.name, args))
                {
                    sub_commands.push(tokens);
                } else {
                    other_items.push(Item::Mod(item))
                }
            },
            Item::Use(mut item) => {
                if let Some(Some(paths)) = acc.handle(use_include(&mut item)) {
                    sub_commands.extend(paths.into_iter().map(|t| quote::quote! { #t() }));
                } else {
                    other_items.push(Item::Use(item));
                }
            },
            item => other_items.push(item),
        }
    }

    if sub_commands.is_empty() {
        let err = Error::custom(
            "command group must have at least one #[sub_command] `fn`, `mod`, or `use`",
        );
        acc.push(err.with_span(&module.ident));
    }

    let name = name.unwrap_or_else(|| module.ident.unraw().to_string());
    let description = acc.handle(extract_description(&module.attrs).ok_or_else(|| {
        Error::custom("a description is required, add a doc comment").with_span(&module.ident)
    }));

    if !(1..=32).contains(&name.chars().count()) {
        let err = Error::custom("the name must be 1 to 32 characters long");
        acc.push(err.with_span(&module.ident));
    }
    if let Some(description) = &description
        && !(1..=100).contains(&description.chars().count())
    {
        let err = Error::custom("the description must be 1 to 100 characters long");
        acc.push(err.with_span(&description.span()));
    }
    if !(1..=25).contains(&sub_commands.len()) {
        let err = Error::custom("there must be 1 to 25 sub commands");
        acc.push(err.with_span(&module.ident));
    }

    let CommonArgs { crate_ } = args;
    let description = description.as_ref().map(|s| &***s).unwrap_or_default();
    let errors = acc.finish().err().map(|e| e.write_errors());

    Ok(quote::quote! {{
        #errors
        #(#warnings)*
        #(#other_items)*

        #crate_::model::CommandOption::builder()
            .name(::std::borrow::Cow::Borrowed(#name))
            .description(::std::borrow::Cow::Borrowed(#description))
            .data(#crate_::model::CommandOptionData::Group(#crate_::model::GroupData::builder()
                // this const-block is necessary to satisfy the compiler when the list
                // involves function calls in place of a sub-command struct literal
                .sub_commands(::std::borrow::Cow::Borrowed(const { &[
                    #(#sub_commands),*
                ] }))
                .build()
            ))
            .build()
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

fn use_include(item: &mut ItemUse) -> darling::Result<Option<Vec<TokenStream>>> {
    let Some(attr) = find_sub_command_attr(&mut item.attrs) else {
        return Ok(None);
    };

    if !matches!(attr.meta, Meta::Path(_)) {
        let err = Error::custom("`#[sub_command] use` cannot specify additional parameters");
        return Err(err.with_span(&attr));
    }

    fn resolve_tree(
        buf: &mut Vec<TokenStream>,
        prefix: Option<&dyn ToTokens>,
        tree: &UseTree,
    ) -> darling::Result<()> {
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
            UseTree::Rename(_) => {
                Err(Error::custom("cannot rename `#[sub_command] use`").with_span(&tree))
            },
            UseTree::Glob(_) => {
                Err(Error::custom("cannot glob-import `#[sub_command] use`").with_span(&tree))
            },
            UseTree::Group(group) => {
                for pair in group.items.pairs() {
                    let item = pair.into_value();
                    resolve_tree(buf, prefix, item)?;
                }
                Ok(())
            },
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
