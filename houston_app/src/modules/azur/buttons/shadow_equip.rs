use azur_lane::equip::*;
use azur_lane::ship::*;
use utils::text::WriteStr as _;

use super::ship::View as ShipView;
use super::{AzurParseError, acknowledge_unloaded};
use crate::buttons::prelude::*;
use crate::config::emoji;
use crate::modules::azur::LoadedConfig;

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

    fn create_with_ship<'a>(
        self,
        azur: LoadedConfig<'a>,
        ship: &'a ShipData,
        base_ship: Option<&'a ShipData>,
    ) -> CreateReply<'a> {
        let base_ship = base_ship.unwrap_or(ship);

        let mut embed = CreateEmbed::new()
            .author(azur.wiki_urls().ship(base_ship))
            .color(ship.rarity.color_rgb());

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

        for mount in &ship.shadow_equip {
            if let Some(value) = format_weapons(&mount.weapons) {
                embed = embed.field(
                    format!("**`{: >3.0}%`** {}", mount.efficiency * 100f64, mount.name),
                    value,
                    true,
                );
            }
        }

        for equip in &ship.depth_charges {
            if let Some(value) = format_weapons(&equip.weapons) {
                embed = embed.field(format!("**`ASW:`** {}", equip.name), value, true);
            }
        }

        let components = vec![CreateActionRow::buttons(vec![{
            let back = self.inner.to_custom_id();
            CreateButton::new(back).emoji(emoji::back()).label("Back")
        }])];

        CreateReply::new().embed(embed).components(components)
    }
}

button_value!(View<'_>, 6);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        acknowledge_unloaded(&ctx).await?;

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
            None => self.create_with_ship(azur, ship, None),
            Some(retrofit) => self.create_with_ship(azur, retrofit, Some(ship)),
        };

        ctx.edit(create.into()).await
    }
}
