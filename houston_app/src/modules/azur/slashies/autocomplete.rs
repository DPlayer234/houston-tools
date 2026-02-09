use crate::slashies::prelude::*;

macro_rules! make_autocomplete {
    ($fn_name:ident, $by_prefix:ident, $val:ident, $id:expr, $name:expr) => {
        pub async fn $fn_name<'a>(
            ctx: Context<'a>,
            partial: &'a str,
        ) -> CreateAutocompleteResponse<'a> {
            if let Ok(azur) = ctx.data_ref().config().azur() {
                let choices: Vec<_> = azur
                    .game_data()
                    .$by_prefix(partial)
                    .take(25)
                    .map(|$val| AutocompleteChoice::new($name, Cow::Owned(format!("/id:{}", $id))))
                    .collect();

                CreateAutocompleteResponse::new().set_choices(choices)
            } else {
                CreateAutocompleteResponse::new()
            }
        }
    };
}

make_autocomplete!(ship_name, ships_by_prefix, e, e.base.group_id, &e.base.name);
make_autocomplete!(equip_name, equips_by_prefix, e, e.equip_id, &e.name);
make_autocomplete!(augment_name, augments_by_prefix, e, e.augment_id, &e.name);
make_autocomplete!(
    special_secretary_name,
    special_secretaries_by_prefix,
    e,
    e.id,
    &e.name
);

pub async fn ship_name_juustagram_chats<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> CreateAutocompleteResponse<'a> {
    if let Ok(azur) = ctx.data_ref().config().azur() {
        let choices: Vec<_> = azur
            .game_data()
            .ships_by_prefix(partial)
            .filter(|s| {
                azur.game_data()
                    .juustagram_chats_by_ship_id(s.base.group_id)
                    .next()
                    .is_some()
            })
            .take(25)
            .map(|e| {
                AutocompleteChoice::new(
                    &e.base.name,
                    Cow::Owned(format!("/id:{}", e.base.group_id)),
                )
            })
            .collect();

        CreateAutocompleteResponse::new().set_choices(choices)
    } else {
        CreateAutocompleteResponse::new()
    }
}
