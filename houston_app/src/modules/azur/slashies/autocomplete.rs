use std::borrow::Cow;

use serenity::builder::{AutocompleteChoice, CreateAutocompleteResponse};

use houston_cmd::Context;

use crate::slashies::ContextExt;

macro_rules! make_autocomplete {
    ($fn_name:ident, $by_prefix:ident, $id:ident) => {
        pub async fn $fn_name<'a>(ctx: Context<'a>, partial: &'a str) -> CreateAutocompleteResponse<'a> {
            let choices: Vec<_> = ctx
                .data_ref()
                .azur_lane()
                .$by_prefix(partial)
                .take(25)
                .map(|e| AutocompleteChoice::new(e.name.as_str(), Cow::Owned(format!("/id:{}", e.$id))))
                .collect();

            CreateAutocompleteResponse::new()
                .set_choices(choices)
        }
    };
}

make_autocomplete!(ship_name, ships_by_prefix, group_id);
make_autocomplete!(equip_name, equips_by_prefix, equip_id);
make_autocomplete!(augment_name, augments_by_prefix, augment_id);
