//! Proc macros for the `houston_cmd` crate.

use proc_macro::TokenStream as StdTokenStream;
use syn::DeriveInput;

mod args;
mod chat_command;
mod context_command;
mod derive_choice_arg;
mod shared_command;
mod util;

/// Turns a function into a chat command or a module into a chat command group.
///
/// See the docs on the `houston_cmd` crate.
#[proc_macro_attribute]
pub fn chat_command(args: StdTokenStream, item: StdTokenStream) -> StdTokenStream {
    chat_command::entry_point(args.into(), item.into())
        .unwrap_or_else(|e| e.write_errors())
        .into()
}

/// Turns a function into a context menu command.
/// This function must have 2 parameters: The context and the relevant item.
///
/// See the docs on the `houston_cmd` crate.
#[proc_macro_attribute]
pub fn context_command(args: StdTokenStream, item: StdTokenStream) -> StdTokenStream {
    context_command::entry_point(args.into(), item.into())
        .unwrap_or_else(|e| e.write_errors())
        .into()
}

/// Turns a function into a chat command or a module into a chat command group
/// _option_.
///
/// This is intended for use when you want to programatically create command
/// groups or to be later `use`d in a command group.
#[proc_macro_attribute]
pub fn sub_command(args: StdTokenStream, item: StdTokenStream) -> StdTokenStream {
    chat_command::sub_entry_point(args.into(), item.into())
        .unwrap_or_else(|e| e.write_errors())
        .into()
}

/// Derives [`ChoiceArg`] for an enum.
#[proc_macro_derive(ChoiceArg, attributes(name, choice_arg))]
pub fn derive_choice_arg(input: StdTokenStream) -> StdTokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    derive_choice_arg::entry_point(input)
        .unwrap_or_else(|e| e.write_errors())
        .into()
}
