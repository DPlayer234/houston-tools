//! Proc macros for the `bson_model` crate.

use proc_macro::TokenStream as StdTokenStream;
use syn::DeriveInput;

mod args;
mod model_impl;

/// Derives the `ModelDocument` trait based on the structure.
#[proc_macro_derive(ModelDocument, attributes(model))]
pub fn derive_model_document(input: StdTokenStream) -> StdTokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    model_impl::entry_point(input)
        .unwrap_or_else(|e| e.write_errors())
        .into()
}
