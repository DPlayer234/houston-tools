use serenity::all::AutocompleteChoice;

use crate::data::HContext;

macro_rules! make_autocomplete {
    ($fn_name:ident, $by_prefix:ident, $id:ident) => {
        pub async fn $fn_name<'a>(ctx: HContext<'a>, partial: &'a str) -> impl Iterator<Item = AutocompleteChoice> + 'a {
            ctx.data().azur_lane()
                .$by_prefix(partial)
                .map(|e| AutocompleteChoice::new(e.name.as_str(), format!("/id:{}", e.$id)))
        }
    };
}

make_autocomplete!(ship_name, ships_by_prefix, group_id);
make_autocomplete!(equip_name, equips_by_prefix, equip_id);
make_autocomplete!(augment_name, augments_by_prefix, augment_id);
