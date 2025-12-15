use darling::util::SpannedValue;
use syn::punctuated::Punctuated;
use syn::{Ident, Lit, LitInt, Path, Token};

#[derive(Debug, darling::FromMeta)]
pub struct ChatCommandArgs {
    pub name: Option<String>,
    #[darling(flatten)]
    pub main: AnyCommandArgs,
}

#[derive(Debug, darling::FromMeta)]
pub struct ContextCommandArgs {
    #[darling(default)]
    pub user: bool,
    #[darling(default)]
    pub message: bool,
    pub name: String,
    #[darling(flatten)]
    pub main: AnyCommandArgs,
}

#[derive(Debug, darling::FromMeta)]
pub struct AnyCommandArgs {
    pub default_member_permissions: Option<Punctuated<Ident, Token![|]>>,
    pub contexts: Option<Punctuated<Ident, Token![|]>>,
    pub integration_types: Option<Punctuated<Ident, Token![|]>>,
    #[darling(default)]
    pub nsfw: bool,
    #[darling(flatten)]
    pub common: CommonArgs,
}

#[derive(Debug, darling::FromMeta)]
pub struct ParameterArgs {
    pub name: Option<String>,
    pub doc: SpannedValue<String>,
    pub autocomplete: Option<Path>,
    pub min: Option<Lit>,
    pub max: Option<Lit>,
    pub min_length: Option<LitInt>,
    pub max_length: Option<LitInt>,
}

#[derive(Debug, darling::FromMeta)]
pub struct TopSubCommandArgs {
    pub name: Option<String>,
    #[darling(flatten)]
    pub common: CommonArgs,
}

#[derive(Debug, Default, darling::FromMeta)]
pub struct SubCommandArgs {
    pub name: Option<String>,
}

#[derive(Debug, darling::FromDeriveInput)]
#[darling(attributes(choice_arg))]
pub struct ChoiceArgArgs {
    #[darling(flatten)]
    pub common: CommonArgs,
}

#[derive(Debug, darling::FromMeta)]
pub struct ChoiceArgVariantArgs {
    pub name: Option<String>,
}

fn default_crate() -> Path {
    syn::parse_quote!(::houston_cmd)
}

#[derive(Debug, darling::FromMeta)]
pub struct CommonArgs {
    #[darling(rename = "crate")]
    #[darling(default = "default_crate")]
    pub crate_: Path,
}
