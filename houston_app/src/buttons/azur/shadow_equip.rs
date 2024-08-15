use std::fmt::Write;

use azur_lane::equip::*;
use azur_lane::ship::*;
use utils::Discard;

use crate::buttons::*;
use super::ShipParseError;
use super::ship::View as ShipView;

/// View a ship's shadow equip.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub inner: ShipView,
}

impl View {
    pub fn new(inner: ShipView) -> Self {
        Self {
            inner
        }
    }

    pub fn modify_with_ship(self, create: CreateReply, ship: &ShipData, base_ship: Option<&ShipData>) -> CreateReply {
        let base_ship = base_ship.unwrap_or(ship);

        let mut embed = CreateEmbed::new()
            .author(super::get_ship_wiki_url(base_ship))
            .color(ship.rarity.color_rgb());

        fn format_weapons(weapons: &[Weapon]) -> Option<String> {
            if weapons.is_empty() {
                return None;
            }

            let mut value = String::new();
            for weapon in weapons {
                write!(value, "{}\n\n", crate::fmt::azur::DisplayWeapon::new(weapon)).discard();
            }

            Some(value)
        }

        for mount in &ship.shadow_equip {
            if let Some(value) = format_weapons(&mount.weapons) {
                embed = embed.field(
                    format!("**`{: >3.0}%`** {}", mount.efficiency * 100f64, mount.name),
                    value,
                    true
                );
            }
        }

        for equip in &ship.depth_charges {
            if let Some(value) = format_weapons(&equip.weapons) {
                embed = embed.field(
                    format!("**`ASW:`** {}", equip.name),
                    value,
                    true
                );
            }
        }

        let components = vec![
            CreateActionRow::Buttons(vec![{
                let back = self.inner.to_custom_id();
                CreateButton::new(back).emoji('⏪').label("Back")
            }])
        ];

        create.embed(embed).components(components)
    }
}

impl ButtonMessage for View {
    fn create_reply(self, ctx: ButtonContext<'_>) -> anyhow::Result<CreateReply> {
        let ship = ctx.data.azur_lane().ship_by_id(self.inner.ship_id).ok_or(ShipParseError)?;
        Ok(match self.inner.retrofit.and_then(|index| ship.retrofits.get(usize::from(index))) {
            None => self.modify_with_ship(ctx.create_reply(), ship, None),
            Some(retrofit) => self.modify_with_ship(ctx.create_reply(), retrofit, Some(ship))
        })
    }
}
