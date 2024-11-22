use darling::ast::NestedMeta;
use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::TokenStreamExt;
use syn::ext::IdentExt;
use syn::{FnArg, ItemFn, Pat, Type};

use crate::args::ParameterArgs;
use crate::util::{ensure_spanned, extract_description, quote_map_option};

struct Parameter {
    name: String,
    args: ParameterArgs,
    ty: Box<Type>,
}

pub fn to_command_option_command(func: &mut ItemFn, name: Option<String>) -> syn::Result<TokenStream> {
    let parameters = extract_parameters(func)?;

    let param_names: Vec<_> = parameters.iter()
        .map(|param| &param.name)
        .collect();

    let param_tys: Vec<_> = parameters.iter()
        .map(|param| &*param.ty)
        .collect();

    let param_idents: Vec<_> = parameters
        .iter()
        .enumerate()
        .map(|(index, _)| quote::format_ident!("param_{index}"))
        .collect();

    let param_data: Vec<_> = parameters
        .iter()
        .map(to_command_parameter)
        .collect();

    let func_ident = &func.sig.ident;
    let name = name.unwrap_or_else(|| func.sig.ident.unraw().to_string());
    let description = extract_description(&func.attrs)
        .ok_or_else(|| syn::Error::new_spanned(&func, "a description is required, add a doc comment"))?;

    ensure_spanned!(func, (1..=32).contains(&name.chars().count()) => "the name must be 1 to 32 characters long");
    ensure_spanned!(func, (1..=100).contains(&description.chars().count()) => "the description must be 1 to 100 characters long");
    ensure_spanned!(func, (0..=25).contains(&parameters.len()) => "there must be at most 25 parameters");

    Ok(quote::quote! {
        ::houston_cmd::model::CommandOption {
            name: ::std::borrow::Cow::Borrowed(#name),
            description: ::std::borrow::Cow::Borrowed(#description),
            data: ::houston_cmd::model::CommandOptionData::Command(::houston_cmd::model::SubCommandData {
                invoke: {
                    #func

                    ::houston_cmd::model::Invoke::ChatInput(|ctx| ::std::boxed::Box::pin(async move {
                        #(
                            let #param_idents = ::houston_cmd::parse_slash_argument!(ctx, #param_names, #param_tys);
                        )*

                        #func_ident (ctx, #(#param_idents),*)
                            .await
                            .map_err(|e| ::houston_cmd::Error::command(ctx, e))
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
    for input in func.sig.inputs.iter_mut().skip(1) {
        let input = match input {
            FnArg::Typed(x) => x,
            FnArg::Receiver(receiver) => return Err(syn::Error::new_spanned(receiver, "invalid self argument")),
        };

        let args = input.attrs
            .drain(..)
            .map(|a| NestedMeta::Meta(a.meta))
            .collect::<Vec<_>>();
        let args = ParameterArgs::from_list(&args)?;

        let name = if let Some(name) = &args.name {
            name.clone()
        } else if let Pat::Ident(ident) = &*input.pat {
            ident.ident.unraw().to_string()
        } else {
            return Err(syn::Error::new_spanned(&input.pat, "#[name = ...] must be specified for pattern parameters"));
        };

        ensure_spanned!(input, (1..=32).contains(&name.chars().count()) => "the name must be 1 to 32 characters long");
        ensure_spanned!(input, (1..=100).contains(&args.description.chars().count()) => "the description must be 1 to 100 characters long");

        parameters.push(Parameter {
            name,
            args,
            ty: input.ty.clone(),
        });
    }

    Ok(parameters)
}

fn to_command_parameter(p: &Parameter) -> TokenStream {
    let name = &p.name;
    let description = &p.args.description;
    let ty = &*p.ty;
    let autocomplete = quote_map_option(p.args.autocomplete.as_ref(), |a| quote::quote! { |ctx, partial| ::std::boxed::Box::pin(#a(ctx, partial)) });

    let mut setter = quote::quote! {};
    if let Some(m) = &p.args.min { setter.append_all(quote::quote! { .min_number_value(#m as f64) }); }
    if let Some(m) = &p.args.max { setter.append_all(quote::quote! { .max_number_value(#m as f64) }); }
    if let Some(m) = &p.args.min_length { setter.append_all(quote::quote! { .min_length(#m) }); }
    if let Some(m) = &p.args.max_length { setter.append_all(quote::quote! { .max_length(#m) }); }

    quote::quote! {
        ::houston_cmd::model::Parameter {
            name: ::std::borrow::Cow::Borrowed(#name),
            description: ::std::borrow::Cow::Borrowed(#description),
            autocomplete: #autocomplete,
            .. ::houston_cmd::create_slash_argument!(#ty, #setter)
        }
    }
}
