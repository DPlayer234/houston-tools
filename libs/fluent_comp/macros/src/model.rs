use std::collections::HashMap;

use syn::Path;

#[derive(Debug, darling::FromMeta)]
#[darling(derive_syn_parse)]
pub struct BundleInput {
    pub locales: Path,
    pub default: String,
    #[darling(flatten)]
    pub resources: HashMap<String, String>,
}
