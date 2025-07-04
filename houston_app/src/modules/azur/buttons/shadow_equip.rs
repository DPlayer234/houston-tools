use azur_lane::equip::*;
use azur_lane::ship::*;
use utils::text::WriteStr as _;

use super::AzurParseError;
use super::ship::View as ShipView;
use crate::buttons::prelude::*;
use crate::config::emoji;

/// View a ship's shadow equip.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View<'v> {
    #[serde(borrow)]
    pub inner: ShipView<'v>,
}

impl<'v> View<'v> {
    pub fn new(inner: ShipView<'v>) -> Self {
        Self { inner }
    }

    fn create_with_ship(self, ship: &ShipData) -> CreateReply<'_> {
        fn format_weapons(weapons: &[Weapon]) -> Option<String> {
            if weapons.is_empty() {
                return None;
            }

            let mut value = String::new();
            for weapon in weapons {
                write!(value, "{}\n\n", crate::fmt::azur::Details::new(weapon));
            }

            Some(value)
        }

        let mut components = CreateComponents::new();

        components.push(CreateTextDisplay::new(format!(
            "### {} [Shadow Equip]",
            ship.name
        )));

        components.push(CreateSeparator::new(true));

        for mount in &ship.shadow_equip {
            if let Some(value) = format_weapons(&mount.weapons) {
                let label = format!(
                    "### **`{: >3.0}%`** {}",
                    mount.efficiency * 100f64,
                    mount.name
                );

                components.push(CreateTextDisplay::new(label));
                components.push(CreateTextDisplay::new(value));
                components.push(CreateSeparator::new(true));
            }
        }

        for equip in &ship.depth_charges {
            if let Some(value) = format_weapons(&equip.weapons) {
                let label = format!("### `ASW:` {}", equip.name);
                components.push(CreateTextDisplay::new(label));
                components.push(CreateTextDisplay::new(value));
                components.push(CreateSeparator::new(true));
            }
        }

        components.push(CreateActionRow::buttons(vec![{
            let back = self.inner.to_custom_id();
            CreateButton::new(back).emoji(emoji::back()).label("Back")
        }]));

        CreateReply::new().components_v2(components![
            CreateContainer::new(components).accent_color(ship.rarity.color_rgb())
        ])
    }
}

button_value!(for<'v> View<'v>, 6);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        let azur = ctx.data.config().azur()?;
        let ship = azur
            .game_data()
            .ship_by_id(self.inner.ship_id)
            .ok_or(AzurParseError::Ship)?;

        let create = match self
            .inner
            .retrofit
            .and_then(|index| ship.retrofits.get(usize::from(index)))
        {
            None => self.create_with_ship(ship),
            Some(retrofit) => self.create_with_ship(retrofit),
        };

        ctx.edit(create.into()).await
    }
}
