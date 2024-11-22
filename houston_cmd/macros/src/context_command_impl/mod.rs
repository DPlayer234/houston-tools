use darling::FromMeta;
use proc_macro2::TokenStream;
use syn::spanned::Spanned;
use syn::{FnArg, ItemFn};

use crate::any_command_impl::to_command_shared;
use crate::args::ContextCommandArgs;

enum ContextKind {
    User,
    Message,
}

pub fn entry_point(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args_span = args.span();
    let args = darling::ast::NestedMeta::parse_meta_list(args)?;
    let args = ContextCommandArgs::from_list(&args)?;

    let func: ItemFn = syn::parse2(item)?;
    if func.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(func.sig, "command function must be async"));
    }

    let kind = match (args.user, args.message) {
        (true, false) => ContextKind::User,
        (false, true) => ContextKind::Message,
        _ => return Err(syn::Error::new(args_span, "must specify `user` or `message`")),
    };

    let command_option = to_command_option_command(&func, args.name, kind)?;
    to_command_shared(&func.vis, &func.sig.ident, command_option, args.main)
}

fn to_command_option_command(func: &ItemFn, name: String, kind: ContextKind) -> syn::Result<TokenStream> {
    let (kind_variant, kind_trait, kind_args) = match kind {
        ContextKind::User => (quote::format_ident!("User"), quote::format_ident!("UserContextArg"), quote::quote! { user, member }),
        ContextKind::Message => (quote::format_ident!("Message"), quote::format_ident!("MessageContextArg"), quote::quote! { message }),
    };

    let func_ident = &func.sig.ident;

    let inputs: Vec<_> = func.sig.inputs.iter().collect();
    let &[_, arg] = inputs.as_slice() else {
        return Err(syn::Error::new_spanned(&func.sig, "expected exacty 1 command argument"));
    };

    let arg = match arg {
        FnArg::Receiver(receiver) => return Err(syn::Error::new_spanned(receiver, "invalid self argument")),
        FnArg::Typed(x) => x,
    };

    let arg_ty = &arg.ty;

    Ok(quote::quote! {
        ::houston_cmd::model::CommandOption {
            name: ::std::borrow::Cow::Borrowed(#name),
            description: ::std::borrow::Cow::Borrowed(""),
            data: ::houston_cmd::model::CommandOptionData::Command(::houston_cmd::model::SubCommandData {
                invoke: {
                    #func

                    ::houston_cmd::model::Invoke:: #kind_variant (|ctx, #kind_args| ::std::boxed::Box::pin(async move {
                        let arg = <#arg_ty as ::houston_cmd:: #kind_trait <'_>>::extract(&ctx, #kind_args)?;

                        #func_ident (ctx, arg)
                            .await
                            .map_err(|e| ::houston_cmd::Error::command(ctx, e))
                    }))
                },
                parameters: ::std::borrow::Cow::Borrowed(&[]),
            }),
        }
    })
}
