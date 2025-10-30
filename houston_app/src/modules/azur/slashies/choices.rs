use azur_lane::Faction;
use azur_lane::equip::{AugmentRarity, EquipKind, EquipRarity};
use azur_lane::ship::{HullType, ShipRarity, TeamType};
use houston_cmd::{Error, SlashArg};
use houston_utils::contains_ignore_ascii_case;

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
            let choices: Vec<_> = (0u64..)
                .zip(<$Type>::ALL)
                .filter(|(_, t)| contains_ignore_ascii_case(t.name(), partial))
                .take(25)
                .map(|(i, t)| AutocompleteChoice::new(t.name(), AutocompleteValue::Integer(i)))
                .collect();

            CreateAutocompleteResponse::new().set_choices(choices)
        }

        impl<'ctx> SlashArg<'ctx> for Ch<$Type> {
            fn extract(
                ctx: &Context<'ctx>,
                resolved: &ResolvedValue<'ctx>,
            ) -> Result<Self, Error<'ctx>> {
                match resolved {
                    ResolvedValue::Integer(index) => usize::try_from(*index)
                        .ok()
                        .and_then(|i| <$Type>::ALL.get(i))
                        .map(|&f| Self(f))
                        .ok_or_else(|| Error::arg_invalid(*ctx, "invalid argument index")),
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

#[derive(Clone, Copy)]
pub enum HullOrTeam {
    Hull(HullType),
    Team(TeamType),
}

impl HullOrTeam {
    pub fn name(self) -> &'static str {
        match self {
            Self::Hull(h) => h.name(),
            Self::Team(t) => t.name(),
        }
    }

    pub fn hull_type(self) -> Option<HullType> {
        match self {
            Self::Hull(h) => Some(h),
            _ => None,
        }
    }

    pub fn team_type(self) -> Option<TeamType> {
        match self {
            Self::Team(t) => Some(t),
            _ => None,
        }
    }

    fn from_index(num: usize) -> Option<Self> {
        let index = num >> 1;
        if num & 1 == 0 {
            Some(Self::Hull(*HullType::ALL.get(index)?))
        } else {
            Some(Self::Team(*TeamType::ALL.get(index)?))
        }
    }
}

pub async fn hull_or_team_type<'a>(
    _ctx: Context<'a>,
    partial: &'a str,
) -> CreateAutocompleteResponse<'a> {
    let hull = HullType::ALL.iter().map(|&h| HullOrTeam::Hull(h));
    let hull = (0u64..).map(|n| n << 1).zip(hull);

    let team = TeamType::ALL.iter().map(|&t| HullOrTeam::Team(t));
    let team = (0u64..).map(|n| (n << 1) | 1).zip(team);

    let all = team.chain(hull);

    let choices: Vec<_> = all
        .filter(|(_, t)| contains_ignore_ascii_case(t.name(), partial))
        .take(25)
        .map(|(i, t)| AutocompleteChoice::new(t.name(), AutocompleteValue::Integer(i)))
        .collect();

    CreateAutocompleteResponse::new().set_choices(choices)
}

impl<'ctx> SlashArg<'ctx> for HullOrTeam {
    fn extract(ctx: &Context<'ctx>, resolved: &ResolvedValue<'ctx>) -> Result<Self, Error<'ctx>> {
        match *resolved {
            ResolvedValue::Integer(index) => usize::try_from(index)
                .ok()
                .and_then(Self::from_index)
                .ok_or_else(|| Error::arg_invalid(*ctx, "invalid argument index")),
            _ => Err(Error::structure_mismatch(*ctx, "expected integer")),
        }
    }

    fn set_options(option: CreateCommandOption<'_>) -> CreateCommandOption<'_> {
        option.kind(CommandOptionType::Integer)
    }
}
