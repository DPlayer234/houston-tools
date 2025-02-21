use darling::FromMeta;
use darling::ast::NestedMeta;
use proc_macro2::{Span, TokenStream};
use quote::TokenStreamExt;
use syn::ext::IdentExt;
use syn::fold::Fold;
use syn::spanned::Spanned;
use syn::{FnArg, ItemFn, Pat, Type};

use crate::args::ParameterArgs;
use crate::util::{
    ReplaceLifetimes, ensure_span, ensure_spanned, extract_description, quote_map_option,
};

struct Parameter {
    span: Span,
    name: String,
    args: ParameterArgs,
    ty: Box<Type>,
}

pub fn to_command_option_command(
    func: &mut ItemFn,
    name: Option<String>,
) -> syn::Result<TokenStream> {
    if func.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(
            &func.sig,
            "command function must be async",
        ));
    }

    let parameters = extract_parameters(func)?;

    let func_ident = &func.sig.ident;
    let name = name.unwrap_or_else(|| func.sig.ident.unraw().to_string());
    let description = extract_description(&func.attrs).ok_or_else(|| {
        syn::Error::new_spanned(&func, "a description is required, add a doc comment")
    })?;

    ensure_spanned!(func_ident, (1..=32).contains(&name.chars().count()) => "the name must be 1 to 32 characters long");
    ensure_span!(description.span(), (1..=100).contains(&description.chars().count()) => "the description must be 1 to 100 characters long");
    ensure_spanned!(&func.sig.inputs, (0..=25).contains(&parameters.len()) => "there must be at most 25 parameters");

    let description = &*description;

    let param_data: Vec<_> = parameters.iter().map(to_command_parameter).collect();

    let param_idents: Vec<_> = parameters
        .iter()
        .enumerate()
        .map(|(index, _)| quote::format_ident!("param_{index}"))
        .collect();

    let param_quotes = parameters.iter().zip(&param_idents).map(|(param, param_ident)| {
        let param_name = &param.name;
        let param_ty = &*param.ty;
        quote::quote_spanned! {param.span=>
            let #param_ident = ::houston_cmd::parse_slash_argument!(ctx, #param_name, #param_ty)?;
        }
    });

    Ok(quote::quote_spanned! {func.sig.output.span()=>
        ::houston_cmd::model::CommandOption {
            name: ::std::borrow::Cow::Borrowed(#name),
            description: ::std::borrow::Cow::Borrowed(#description),
            data: ::houston_cmd::model::CommandOptionData::Command(::houston_cmd::model::SubCommandData {
                invoke: {
                    #func

                    ::houston_cmd::model::Invoke::ChatInput(|ctx| ::std::boxed::Box::pin(async move {
                        #( #param_quotes )*

                        match #func_ident (ctx, #(#param_idents),*).await {
                            ::std::result::Result::Ok(()) => ::std::result::Result::Ok(()),
                            ::std::result::Result::Err(e) => ::std::result::Result::Err(::houston_cmd::Error::command(ctx, e)),
                        }
                    }))
                },
                parameters: ::std::borrow::Cow::Borrowed(&[
                    #(#param_data),*
                ]),
            }),
        }
    })
}

fn extract_parameters(func: &mut ItemFn) -> syn::Result<Vec<Parameter>> {
    let mut parameters = Vec::new();

    let mut fold_type = ReplaceLifetimes::omit();
    for input in func.sig.inputs.iter_mut().skip(1) {
        let input = match input {
            FnArg::Typed(x) => x,
            FnArg::Receiver(receiver) => {
                return Err(syn::Error::new_spanned(receiver, "invalid self argument"));
            },
        };

        let args = input
            .attrs
            .drain(..)
            .map(|a| NestedMeta::Meta(a.meta))
            .collect::<Vec<_>>();
        let args = ParameterArgs::from_list(&args).map_err(|e| e.with_span(&input))?;
        let span = input.span();

        let name = if let Some(name) = &args.name {
            name.clone()
        } else if let Pat::Ident(ident) = &*input.pat {
            ident.ident.unraw().to_string()
        } else {
            return Err(syn::Error::new_spanned(
                &input.pat,
                "#[name = ...] must be specified for pattern parameters",
            ));
        };

        ensure_spanned!(&input.pat, (1..=32).contains(&name.chars().count()) => "the name must be 1 to 32 characters long");
        ensure_span!(args.doc.span(), (1..=100).contains(&args.doc.chars().count()) => "the description must be 1 to 100 characters long");

        parameters.push(Parameter {
            span,
            name,
            args,
            ty: fold_type.fold_type((*input.ty).clone()).into(),
        });
    }

    Ok(parameters)
}

fn to_command_parameter(p: &Parameter) -> TokenStream {
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

    quote::quote! {
        ::houston_cmd::create_slash_argument!((
            name: ::std::borrow::Cow::Borrowed(#name),
            description: ::std::borrow::Cow::Borrowed(#description),
            autocomplete: #autocomplete
        ), #ty, #setter)
    }
}
