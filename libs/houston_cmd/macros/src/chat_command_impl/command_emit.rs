use darling::ast::NestedMeta;
use darling::{Error, FromMeta as _};
use proc_macro2::{Span, TokenStream};
use quote::TokenStreamExt as _;
use syn::ext::IdentExt as _;
use syn::fold::Fold as _;
use syn::spanned::Spanned as _;
use syn::{FnArg, ItemFn, Pat, Type};

use crate::args::{CommonArgs, ParameterArgs};
use crate::util::{ReplaceLifetimes, extract_description, quote_map_option};

struct Parameter {
    span: Span,
    name: String,
    args: ParameterArgs,
    ty: Box<Type>,
}

pub fn to_command_option_command(
    func: &mut ItemFn,
    name: Option<String>,
    args: &CommonArgs,
) -> TokenStream {
    let mut acc = Error::accumulator();

    if func.sig.asyncness.is_none() {
        let err = Error::custom("command function must be async");
        acc.push(err.with_span(&func.sig));
    }

    let parameters = extract_parameters(func, &mut acc);

    let func_ident = &func.sig.ident;
    let name = name.unwrap_or_else(|| func.sig.ident.unraw().to_string());

    let description = acc.handle(extract_description(&func.attrs).ok_or_else(|| {
        Error::custom("a description is required, add a doc comment").with_span(func_ident)
    }));

    if !(1..=32).contains(&name.chars().count()) {
        let err = Error::custom("the name must be 1 to 32 characters long");
        acc.push(err.with_span(func_ident));
    }
    if let Some(description) = &description
        && !(1..=100).contains(&description.chars().count())
    {
        let err = Error::custom("the description must be 1 to 100 characters long");
        acc.push(err.with_span(&description.span()));
    }
    if !(0..=25).contains(&parameters.len()) {
        let err = Error::custom("there must be at most 25 parameters");
        acc.push(err.with_span(&func.sig.inputs));
    }

    let CommonArgs { crate_ } = args;
    let description = description.as_ref().map(|s| &***s).unwrap_or_default();

    let param_data: Vec<_> = parameters
        .iter()
        .map(|p| to_command_parameter(p, args))
        .collect();

    let param_idents: Vec<_> = parameters
        .iter()
        .enumerate()
        .map(|(index, _)| quote::format_ident!("param_{index}"))
        .collect();

    let param_quotes = parameters
        .iter()
        .zip(&param_idents)
        .map(|(param, param_ident)| {
            let param_name = &param.name;
            let param_ty = &*param.ty;
            quote::quote_spanned! {param.span=>
                let #param_ident = #crate_::parse_slash_argument!(ctx, #param_name, #param_ty)?;
            }
        });

    let errors = acc.finish().err().map(|e| e.write_errors());
    quote::quote_spanned! {func.sig.output.span()=>
        #crate_::model::CommandOption::builder()
            .name(::std::borrow::Cow::Borrowed(#name))
            .description(::std::borrow::Cow::Borrowed(#description))
            .data(#crate_::model::CommandOptionData::Command(#crate_::model::SubCommandData::builder()
                .invoke({
                    #errors
                    #func

                    #crate_::model::Invoke::ChatInput(|ctx| ::std::boxed::Box::pin(async move {
                        #( #param_quotes )*

                        match #func_ident (ctx, #(#param_idents),*).await {
                            ::std::result::Result::Ok(()) => ::std::result::Result::Ok(()),
                            ::std::result::Result::Err(e) => ::std::result::Result::Err(#crate_::Error::command(ctx, e)),
                        }
                    }))
                })
                .parameters(::std::borrow::Cow::Borrowed(const { &[
                    #(#param_data),*
                ] }))
                .build()
            ))
            .build()
    }
}

fn extract_parameters(func: &mut ItemFn, acc: &mut darling::error::Accumulator) -> Vec<Parameter> {
    let mut parameters = Vec::new();

    let mut fold_type = ReplaceLifetimes::omit();
    for input in func.sig.inputs.iter_mut().skip(1) {
        let input = match input {
            FnArg::Typed(x) => x,
            FnArg::Receiver(receiver) => {
                let err = Error::custom("invalid self argument");
                acc.push(err.with_span(receiver));
                continue;
            },
        };

        let args = input
            .attrs
            .drain(..)
            .map(|a| NestedMeta::Meta(a.meta))
            .collect::<Vec<_>>();

        let Some(args) = acc.handle(ParameterArgs::from_list(&args)) else {
            continue;
        };

        let span = input.span();
        let name = if let Some(name) = &args.name {
            name.clone()
        } else if let Pat::Ident(ident) = &*input.pat {
            ident.ident.unraw().to_string()
        } else {
            let err = Error::custom("#[name = ...] must be specified for pattern parameters");
            acc.push(err.with_span(&input.pat));
            continue;
        };

        if !(1..=32).contains(&name.chars().count()) {
            let err = Error::custom("the name must be 1 to 32 characters long");
            acc.push(err.with_span(&input.pat));
        }

        if !(1..=100).contains(&args.doc.chars().count()) {
            let err = Error::custom("the description must be 1 to 100 characters long");
            acc.push(err.with_span(&args.doc.span()));
        }

        parameters.push(Parameter {
            span,
            name,
            args,
            ty: fold_type.fold_type((*input.ty).clone()).into(),
        });
    }

    parameters
}

fn to_command_parameter(p: &Parameter, args: &CommonArgs) -> TokenStream {
    let name = &p.name;
    let description = p.args.doc.trim();
    let ty = &*p.ty;
    let autocomplete = quote_map_option(
        p.args.autocomplete.as_ref(),
        |a| quote::quote! { |ctx, partial| ::std::boxed::Box::pin(#a(ctx, partial)) },
    );

    let mut setter = quote::quote! {};
    if let Some(m) = &p.args.min {
        setter.append_all(quote::quote_spanned! {m.span()=> .min_number_value(#m as f64) });
    }
    if let Some(m) = &p.args.max {
        setter.append_all(quote::quote_spanned! {m.span()=> .max_number_value(#m as f64) });
    }
    if let Some(m) = &p.args.min_length {
        setter.append_all(quote::quote_spanned! {m.span()=> .min_length(#m) });
    }
    if let Some(m) = &p.args.max_length {
        setter.append_all(quote::quote_spanned! {m.span()=> .max_length(#m) });
    }

    let CommonArgs { crate_ } = args;
    quote::quote! {
        #crate_::create_slash_argument!(#ty, #setter)
            .name(::std::borrow::Cow::Borrowed(#name))
            .description(::std::borrow::Cow::Borrowed(#description))
            .autocomplete(#autocomplete)
            .build()
    }
}
