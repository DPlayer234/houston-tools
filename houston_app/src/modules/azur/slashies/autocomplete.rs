use crate::slashies::prelude::*;

macro_rules! make_autocomplete {
    ($fn_name:ident, $by_prefix:ident, $id:ident) => {
        pub async fn $fn_name<'a>(
            ctx: Context<'a>,
            partial: &'a str,
        ) -> CreateAutocompleteResponse<'a> {
            let choices: Vec<_> = ctx
                .data_ref()
                .azur_lane()
                .$by_prefix(partial)
                .take(25)
                .map(|e| {
                    AutocompleteChoice::new(e.name.as_str(), Cow::Owned(format!("/id:{}", e.$id)))
                })
                .collect();

            CreateAutocompleteResponse::new().set_choices(choices)
        }
    };
}

make_autocomplete!(ship_name, ships_by_prefix, group_id);
make_autocomplete!(equip_name, equips_by_prefix, equip_id);
make_autocomplete!(augment_name, augments_by_prefix, augment_id);

pub async fn ship_name_juustagram_chats<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> CreateAutocompleteResponse<'a> {
    let data = ctx.data_ref().azur_lane();

    let choices: Vec<_> = data
        .ships_by_prefix(partial)
        .filter(|s| {
            data.juustagram_chats_by_ship_id(s.group_id)
                .next()
                .is_some()
        })
        .take(25)
        .map(|e| {
            AutocompleteChoice::new(e.name.as_str(), Cow::Owned(format!("/id:{}", e.group_id)))
        })
        .collect();

    CreateAutocompleteResponse::new().set_choices(choices)
}
