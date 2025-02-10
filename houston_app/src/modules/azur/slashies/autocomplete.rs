use crate::slashies::prelude::*;

macro_rules! make_autocomplete {
    ($fn_name:ident, $by_prefix:ident, $id:ident) => {
        pub async fn $fn_name<'a>(
            ctx: Context<'a>,
            partial: &'a str,
        ) -> CreateAutocompleteResponse<'a> {
            if let Some(config) = &ctx.data_ref().config().azur {
                let choices: Vec<_> = config
                    .game_data()
                    .$by_prefix(partial)
                    .take(25)
                    .map(|e| {
                        AutocompleteChoice::new(
                            e.name.as_str(),
                            Cow::Owned(format!("/id:{}", e.$id)),
                        )
                    })
                    .collect();

                CreateAutocompleteResponse::new().set_choices(choices)
            } else {
                CreateAutocompleteResponse::new()
            }
        }
    };
}

make_autocomplete!(ship_name, ships_by_prefix, group_id);
make_autocomplete!(equip_name, equips_by_prefix, equip_id);
make_autocomplete!(augment_name, augments_by_prefix, augment_id);
make_autocomplete!(special_secretary_name, special_secretaries_by_prefix, id);

pub async fn ship_name_juustagram_chats<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> CreateAutocompleteResponse<'a> {
    if let Some(config) = &ctx.data_ref().config().azur {
        let azur = config.game_data();
        let choices: Vec<_> = azur
            .ships_by_prefix(partial)
            .filter(|s| {
                azur.juustagram_chats_by_ship_id(s.group_id)
                    .next()
                    .is_some()
            })
            .take(25)
            .map(|e| {
                AutocompleteChoice::new(e.name.as_str(), Cow::Owned(format!("/id:{}", e.group_id)))
            })
            .collect();

        CreateAutocompleteResponse::new().set_choices(choices)
    } else {
        CreateAutocompleteResponse::new()
    }
}
