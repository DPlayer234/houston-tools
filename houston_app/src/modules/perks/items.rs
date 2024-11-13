use super::config::{Config, ItemPrice};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq,
    serde::Serialize, serde::Deserialize, poise::ChoiceParameter,
)]
pub enum Item {
    Cash,
    Pushpin,
    Collectible,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ItemInfo<'a> {
    pub name: &'a str,
    pub description: &'a str,
}

impl Item {
    pub fn all() -> &'static [Self] {
        &[
            Self::Cash,
            Self::Pushpin,
            Self::Collectible,
        ]
    }

    pub fn info(self, perks: &Config) -> ItemInfo<'_> {
        macro_rules! extract_or {
            ($expr:expr, $name:literal) => {
                $expr.as_ref()
                    .map(|c| ItemInfo { name: &c.name, description: &c.description })
                    .unwrap_or(ItemInfo { name: $name, description: "<Disabled>" })
            };
        }

        match self {
            Self::Cash => ItemInfo { name: &perks.cash_name, description: "Illegal tender." },
            Self::Pushpin => extract_or!(perks.pushpin, "Pushpin"),
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
            Self::Collectible => extract!(perks.collectible),
        }
    }
}
