use serenity::all::AutocompleteChoice;

use crate::data::HContext;

macro_rules! make_autocomplete {
    ($fn_name:ident, $by_prefix:ident, $id:ident) => {
        pub async fn $fn_name<'a>(ctx: HContext<'a>, partial: &'a str) -> Vec<AutocompleteChoice<'static>> {
            ctx.data().azur_lane()
                .$by_prefix(partial)
                .map(|e| AutocompleteChoice::new(e.name.clone(), format!("/id:{}", e.$id)))
                .collect()
        }
    };
}

make_autocomplete!(ship_name, ships_by_prefix, group_id);
make_autocomplete!(equip_name, equips_by_prefix, equip_id);
make_autocomplete!(augment_name, augments_by_prefix, augment_id);
