use super::config::{Config, ItemPrice};


#[derive(
    Debug, Clone, Copy, PartialEq, Eq,
    serde::Serialize, serde::Deserialize, poise::ChoiceParameter,
)]
pub enum Item {
    Cash,
    Collectible,
}

impl Item {
    pub fn name(self, perks: &Config) -> &str {
        match self {
            Self::Cash => &perks.cash_name,
            Self::Collectible => perks.collectible.as_ref().map(|c| c.name.as_str()).unwrap_or("collectible"),
        }
    }

    pub fn price(self, perks: &Config) -> Option<ItemPrice> {
        match self {
            Self::Cash => None,
            Self::Collectible => perks.collectible.as_ref().map(|c| c.price),
        }
    }
}
