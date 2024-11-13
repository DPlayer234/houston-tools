use super::config::{Config, ItemPrice};


#[derive(
    Debug, Clone, Copy, PartialEq, Eq,
    serde::Serialize, serde::Deserialize, poise::ChoiceParameter,
)]
pub enum Item {
    Cash,
    Collectible,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ItemInfo<'a> {
    pub name: &'a str,
    pub description: &'a str,
}

impl Item {
    pub fn info(self, perks: &Config) -> ItemInfo<'_> {
        match self {
            Self::Cash => ItemInfo { name: &perks.cash_name, description: "Illegal tender." },
            Self::Collectible => perks.collectible.as_ref()
                .map(|c| ItemInfo { name: &c.name, description: &c.description })
                .unwrap_or(ItemInfo { name: "collectible", description: "none" }),
        }
    }

    pub fn price(self, perks: &Config) -> Option<ItemPrice> {
        match self {
            Self::Cash => None,
            Self::Collectible => perks.collectible.as_ref().map(|c| c.price),
        }
    }
}
