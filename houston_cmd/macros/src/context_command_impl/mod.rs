use darling::FromMeta;
use proc_macro2::TokenStream;
use syn::{FnArg, ItemFn};

use crate::any_command_impl::to_command_shared;
use crate::args::ContextCommandArgs;

pub fn entry_point(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args = darling::ast::NestedMeta::parse_meta_list(args)?;
    let args = ContextCommandArgs::from_list(&args)?;

    let func: ItemFn = syn::parse2(item)?;
    if func.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(func.sig, "command function must be async"));
    }

    let command_option = to_command_option_command(&func, args.name)?;
    to_command_shared(&func.vis, &func.sig.ident, command_option, args.main)
}

pub fn to_command_option_command(func: &ItemFn, name: String) -> syn::Result<TokenStream> {
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

                    match <#arg_ty as ::houston_cmd::ContextArg<'_>>::INVOKE {
                        ::houston_cmd::model::Invoke::User(_) => ::houston_cmd::model::Invoke::User(|ctx, user, member| ::std::boxed::Box::pin(async move {
                            let arg = <#arg_ty as ::houston_cmd::ContextArg<'_>>::extract_user(&ctx, user, member)?;

                            #func_ident (ctx, arg)
                                .await
                                .map_err(|e| ::houston_cmd::Error::command(ctx, e))
                        })),
                        ::houston_cmd::model::Invoke::Message(_) => ::houston_cmd::model::Invoke::Message(|ctx, message| ::std::boxed::Box::pin(async move {
                            let arg = <#arg_ty as ::houston_cmd::ContextArg<'_>>::extract_message(&ctx, message)?;

                            #func_ident (ctx, arg)
                                .await
                                .map_err(|e| ::houston_cmd::Error::command(ctx, e))
                        })),
                        _ => unreachable!(),
                    }
                },
                parameters: ::std::borrow::Cow::Borrowed(&[]),
            }),
        }
    })
}
