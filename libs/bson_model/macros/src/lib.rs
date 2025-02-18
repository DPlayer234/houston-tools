use proc_macro::TokenStream as StdTokenStream;
use syn::DeriveInput;

mod args;
mod model_impl;

/// Adds BSON Document builders by deriving the `ModelDocument` trait.
///
/// Additionally, emits a struct for each associated type on `ModelDocument`.
#[proc_macro_derive(ModelDocument, attributes(serde))]
pub fn derive_model_document(input: StdTokenStream) -> StdTokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    model_impl::entry_point(input)
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}
