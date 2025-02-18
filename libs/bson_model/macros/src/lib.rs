use proc_macro::TokenStream as StdTokenStream;
use syn::DeriveInput;

mod args;
mod model_impl;

/// Adds BSON Document builders.
///
/// In particular, it emits three structs. These structs all have chainable
/// setter functions and an `into_document()` function to convert them to a BSON
/// Document.
///
/// ### `<name>Partial`
///
/// Represents a partial document, equivalent to the source type but with all
/// fields wrapped in [`Option`]. This is intended for update operations.
///
/// Call `SourceType::partial()` for an instance. If you intend to use it for
/// more complex updates, use `SourceType::update()` instance, which returns an
/// `Update` builder.
///
/// ### `<name>Filter`
///
/// Represents a filter document. Call `SourceType::filter()` for an instance.
///
/// ### <name>Sort
///
/// Represents a sort document. Call `SourceType::sort()` for an instance.
#[proc_macro_derive(ModelDocument, attributes(serde))]
pub fn derive_builder(input: StdTokenStream) -> StdTokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    model_impl::entry_point(input)
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}
