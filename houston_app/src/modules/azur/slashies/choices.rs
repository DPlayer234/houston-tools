use azur_lane::Faction;
use azur_lane::equip::{AugmentRarity, EquipKind, EquipRarity};
use azur_lane::ship::{HullType, ShipRarity};
use houston_cmd::{Error, SlashArg};

use crate::helper::contains_ignore_case_ascii;
use crate::slashies::prelude::*;

pub struct Ch<T>(T);

impl<T> Ch<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

macro_rules! make_autocomplete_choice {
    ($fn_name:ident, $Type:ty) => {
        pub async fn $fn_name<'a>(
            _ctx: Context<'a>,
            partial: &'a str,
        ) -> CreateAutocompleteResponse<'a> {
            let choices: Vec<_> = <$Type>::ALL
                .iter()
                .enumerate()
                .filter(|(_, t)| contains_ignore_case_ascii(t.name(), partial))
                .take(25)
                .map(|(i, t)| {
                    AutocompleteChoice::new(t.name(), AutocompleteValue::Integer(i as u64))
                })
                .collect();

            CreateAutocompleteResponse::new().set_choices(choices)
        }

        impl<'ctx> SlashArg<'ctx> for Ch<$Type> {
            fn extract(
                ctx: &Context<'ctx>,
                resolved: &ResolvedValue<'ctx>,
            ) -> Result<Self, Error<'ctx>> {
                match resolved {
                    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    ResolvedValue::Integer(index) => <$Type>::ALL
                        .get(*index as usize)
                        .ok_or_else(|| Error::arg_invalid(*ctx, "invalid argument index"))
                        .map(|&f| Self(f)),
                    _ => Err(Error::structure_mismatch(*ctx, "expected integer")),
                }
            }

            fn set_options(option: CreateCommandOption<'_>) -> CreateCommandOption<'_> {
                option.kind(CommandOptionType::Integer)
            }
        }
    };
}

macro_rules! make_choice {
    ($NewType:ident for $OrigType:ident { $($(#[$attr:meta])* $name:ident),* $(,)? }) => {
        #[derive(houston_cmd::ChoiceArg)]
        #[allow(clippy::upper_case_acronyms)]
        pub enum $NewType {
            $(
                $(#[$attr])*
                $name
            ),*
        }

        impl $NewType {
            pub const fn convert(self) -> $OrigType {
                match self {
                    $( Self::$name => $OrigType::$name ),*
                }
            }
        }
    };
}

make_autocomplete_choice!(faction, Faction);
make_autocomplete_choice!(hull_type, HullType);
make_autocomplete_choice!(equip_kind, EquipKind);

make_choice!(EShipRarity for ShipRarity {
    N, R, E, SR, UR,
});

make_choice!(EEquipRarity for EquipRarity {
    #[name = "1* Common"] N1,
    #[name = "2* Common"] N2,
    #[name = "3* Rare"] R,
    #[name = "4* Elite"] E,
    #[name = "5* SR"] SR,
    #[name = "6* UR"] UR,
});

make_choice!(EAugmentRarity for AugmentRarity {
    #[name = "2* Rare"] R,
    #[name = "3* Elite"] E,
    #[name = "4* SR"] SR,
});
