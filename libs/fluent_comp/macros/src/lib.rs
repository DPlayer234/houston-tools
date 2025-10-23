use proc_macro::TokenStream;
use syn::parse_macro_input;

mod bundle_impl;
mod model;
mod util;

#[proc_macro_attribute]
pub fn bundle(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args);
    let input = parse_macro_input!(input);
    bundle_impl::entry_point(args, input)
        .unwrap_or_else(darling::Error::write_errors)
        .into()
}
