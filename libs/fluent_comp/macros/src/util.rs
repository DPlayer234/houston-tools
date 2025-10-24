use std::env;
use std::ffi::OsString;

use quote::format_ident;
use syn::Ident;

pub fn get_manifest_dir() -> OsString {
    env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir must be set")
}

pub fn to_ident(s: &str) -> Ident {
    format_ident!("{}", &s.replace('-', "_"))
}

pub fn to_term_ident(s: &str) -> Ident {
    format_ident!("____term_{}", &s.replace('-', "_"))
}
