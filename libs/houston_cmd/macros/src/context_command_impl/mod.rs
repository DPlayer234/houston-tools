use std::borrow::Cow;

use darling::{Error, FromMeta as _};
use proc_macro2::TokenStream;
use syn::spanned::Spanned as _;
use syn::{FnArg, ItemFn, Type, TypeInfer};

use crate::any_command_impl::to_command_shared;
use crate::args::{CommonArgs, ContextCommandArgs};

enum ContextKind {
    Undefined,
    User,
    Message,
}

pub fn entry_point(args: TokenStream, item: TokenStream) -> darling::Result<TokenStream> {
    let args_span = args.span();
    let args = darling::ast::NestedMeta::parse_meta_list(args)?;
    let args = ContextCommandArgs::from_list(&args)?;

    let func: ItemFn = syn::parse2(item)?;

    let mut acc = Error::accumulator();

    let kind = match (args.user, args.message) {
        (true, false) => ContextKind::User,
        (false, true) => ContextKind::Message,
        _ => {
            let err = Error::custom("must specify `user` or `message`");
            acc.push(err.with_span(&args_span));
            ContextKind::Undefined
        },
    };

    let command_option = to_command_option_command(&func, args.name, kind, &args.main.common, acc);
    Ok(to_command_shared(
        &func.vis,
        &func.sig.ident,
        command_option,
        args.main,
    ))
}

fn to_command_option_command(
    func: &ItemFn,
    name: String,
    kind: ContextKind,
    args: &CommonArgs,
    mut acc: darling::error::Accumulator,
) -> TokenStream {
    let (kind_variant, kind_trait, kind_args) = match kind {
        ContextKind::Undefined => (
            quote::format_ident!("ChatInput"),
            quote::quote! { private::UndefinedContextArg },
            quote::quote! {},
        ),
        ContextKind::User => (
            quote::format_ident!("User"),
            quote::quote! { UserContextArg },
            quote::quote! { user, member },
        ),
        ContextKind::Message => (
            quote::format_ident!("Message"),
            quote::quote! { MessageContextArg },
            quote::quote! { message },
        ),
    };

    let func_ident = &func.sig.ident;

    let maybe_await = func
        .sig
        .asyncness
        .map(|a| quote::quote_spanned! {a.span()=> .await});

    let inputs: Vec<_> = func.sig.inputs.iter().collect();
    let arg_ty = match inputs.as_slice() {
        [_, FnArg::Receiver(receiver)] => {
            let err = Error::custom("invalid self argument");
            acc.push(err.with_span(&receiver));
            Cow::Borrowed(&*receiver.ty)
        },
        [_, FnArg::Typed(x)] => Cow::Borrowed(&*x.ty),
        _ => {
            let err = Error::custom("expected exacty 1 command argument");
            acc.push(err.with_span(&func.sig));
            Cow::Owned(Type::Infer(TypeInfer {
                underscore_token: Default::default(),
            }))
        },
    };

    let CommonArgs { crate_ } = args;
    let errors = acc.finish().err().map(|e| e.write_errors());

    quote::quote_spanned! {func.sig.output.span()=>
        #crate_::model::CommandOption::builder()
            .name(::std::borrow::Cow::Borrowed(#name))
            .data(#crate_::model::CommandOptionData::Command(#crate_::model::SubCommandData::builder()
                .invoke({
                    #errors
                    #func

                    #crate_::model::Invoke:: #kind_variant (|ctx, #kind_args| ::std::boxed::Box::pin(async move {
                        let arg = <#arg_ty as #crate_:: #kind_trait <'_>>::extract(&ctx, #kind_args)?;

                        match #func_ident (ctx, arg) #maybe_await {
                            ::std::result::Result::Ok(()) => ::std::result::Result::Ok(()),
                            ::std::result::Result::Err(e) => ::std::result::Result::Err(#crate_::Error::command(ctx, e)),
                        }
                    }))
                })
                .build()
            ))
            .build()
    }
}
