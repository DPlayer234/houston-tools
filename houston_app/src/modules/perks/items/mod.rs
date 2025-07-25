use super::config::{Config, ItemPrice};
use super::effects::Args;
use crate::modules::prelude::*;

mod collectible;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, houston_cmd::ChoiceArg,
)]
pub enum Item {
    Cash,
    Pushpin,
    RoleEdit,
    Collectible,
}

trait Shape {
    async fn on_buy(&self, args: Args<'_>, from: i64, to: i64) -> Result {
        _ = (args, from, to);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ItemInfo<'a> {
    pub name: &'a str,
    pub description: &'a str,
}

macro_rules! impl_kind_fn {
    ($name:ident ( $($args:ident: $args_ty:ty),* ) -> $ret:ty) => {
        pub async fn $name(self, $($args: $args_ty),*) -> $ret {
            match self {
                Self::Cash | Self::Pushpin | Self::RoleEdit => Ok(()),
                Self::Collectible => collectible::Collectible.$name($($args),*).await,
            }
        }
    };
}

impl Item {
    impl_kind_fn!(on_buy(args: Args<'_>, from: i64, to: i64) -> Result);

    pub fn all() -> &'static [Self] {
        &[Self::Cash, Self::Pushpin, Self::RoleEdit, Self::Collectible]
    }

    pub fn info(self, perks: &Config) -> ItemInfo<'_> {
        macro_rules! extract_or {
            ($expr:expr, $name:literal) => {{
                const DISABLED: ItemInfo<'static> = ItemInfo {
                    name: $name,
                    description: "<Disabled>",
                };
                $expr.as_ref().map_or(DISABLED, |c| ItemInfo {
                    name: &c.name,
                    description: &c.description,
                })
            }};
        }

        match self {
            Self::Cash => ItemInfo {
                name: &perks.cash_name,
                description: "Illegal tender.",
            },
            Self::Pushpin => extract_or!(perks.pushpin, "Pushpin"),
            Self::RoleEdit => extract_or!(perks.role_edit, "Role Edit"),
            Self::Collectible => extract_or!(perks.collectible, "Collectible"),
        }
    }

    pub fn price(self, perks: &Config) -> Option<ItemPrice> {
        macro_rules! extract {
            ($expr:expr) => {
                $expr.as_ref().map(|c| c.price)
            };
        }

        match self {
            Self::Cash => None,
            Self::Pushpin => extract!(perks.pushpin),
            Self::RoleEdit => extract!(perks.role_edit),
            Self::Collectible => extract!(perks.collectible),
        }
    }
}
