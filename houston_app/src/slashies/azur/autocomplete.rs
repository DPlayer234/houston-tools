use serenity::all::AutocompleteChoice;

use crate::data::HContext;

// when poise removes the "magic" here and switches to `CreateAutocompleteResponse`,
// just collect the iter and wrap it in that struct. ensure the `take(25)` stays.
// also revisit whether we can borrow the `e.name` at that point.

macro_rules! make_autocomplete {
    ($fn_name:ident, $by_prefix:ident, $id:ident) => {
        pub async fn $fn_name<'a>(ctx: HContext<'a>, partial: &'a str) -> Vec<AutocompleteChoice<'static>> {
            ctx.data().azur_lane()
                .$by_prefix(partial)
                .take(25)
                .map(|e| AutocompleteChoice::new(e.name.clone(), format!("/id:{}", e.$id)))
                .collect()
        }
    };
}

make_autocomplete!(ship_name, ships_by_prefix, group_id);
make_autocomplete!(equip_name, equips_by_prefix, equip_id);
make_autocomplete!(augment_name, augments_by_prefix, augment_id);
