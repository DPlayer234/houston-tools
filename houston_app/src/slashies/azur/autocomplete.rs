//use std::borrow::Cow;

use serenity::all::{AutocompleteChoice, CreateAutocompleteResponse};

use crate::data::HContext;
//use crate::data::{HContext, HBotData};

// the commented out code here is for when `data_ref` becomes available
// there is currently a PR for it active and it's necessary to return
// borrows from the user data

macro_rules! make_autocomplete {
    ($fn_name:ident, $by_prefix:ident, $id:ident) => {
        pub async fn $fn_name<'a>(ctx: HContext<'a>, partial: &'a str) -> CreateAutocompleteResponse<'a> {
            let choices: Vec<_> = ctx
                .data()
                //.serenity_context()
                //.data_ref::<HBotData>()
                .azur_lane()
                .$by_prefix(partial)
                .take(25)
                .map(|e| AutocompleteChoice::new(e.name.clone(), format!("/id:{}", e.$id)))
                //.map(|e| AutocompleteChoice::new(e.name.as_str(), Cow::Owned(format!("/id:{}", e.$id))))
                .collect();

            CreateAutocompleteResponse::new()
                .set_choices(choices)
        }
    };
}

make_autocomplete!(ship_name, ships_by_prefix, group_id);
make_autocomplete!(equip_name, equips_by_prefix, equip_id);
make_autocomplete!(augment_name, augments_by_prefix, augment_id);
