use proc_macro::TokenStream as StdTokenStream;
use syn::DeriveInput;

mod any_command_impl;
mod args;
mod chat_command_impl;
mod choice_arg_impl;
mod context_command_impl;
mod util;

/// Turns a function into a chat command or a module into a chat command group.
///
/// See the docs on the `houston_cmd` crate.
#[proc_macro_attribute]
pub fn chat_command(args: StdTokenStream, item: StdTokenStream) -> StdTokenStream {
    chat_command_impl::entry_point(args.into(), item.into())
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}

/// Turns a function into a context menu command.
/// This function must have 2 parameters: The context and the relevant item.
///
/// See the docs on the `houston_cmd` crate.
#[proc_macro_attribute]
pub fn context_command(args: StdTokenStream, item: StdTokenStream) -> StdTokenStream {
    context_command_impl::entry_point(args.into(), item.into())
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}

/// Turns a function into a chat command or a module into a chat command group
/// _option_.
///
/// This is intended for use when you want to programatically create command
/// groups or to be later `use`d in a command group.
#[proc_macro_attribute]
pub fn sub_command(args: StdTokenStream, item: StdTokenStream) -> StdTokenStream {
    chat_command_impl::sub_entry_point(args.into(), item.into())
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}

/// Derives [`ChoiceArg`] for an enum.
#[proc_macro_derive(ChoiceArg, attributes(name, choice_arg))]
pub fn derive_choice_arg(input: StdTokenStream) -> StdTokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    choice_arg_impl::entry_point(input)
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}
