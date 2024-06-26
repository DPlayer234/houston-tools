use std::fmt::Write;

use azur_lane::ship::*;
use utils::Discard;

use crate::buttons::*;
use super::ShipParseError;
use super::ship::ViewShip;

/// View a ship's shadow equip.
#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct ViewShadowEquip {
    pub inner: ViewShip,
}

impl From<ViewShadowEquip> for ButtonArgs {
    fn from(value: ViewShadowEquip) -> Self {
        ButtonArgs::ViewShadowEquip(value)
    }
}

impl ViewShadowEquip {
    pub fn new(inner: ViewShip) -> Self {
        Self {
            inner
        }
    }

    pub fn modify_with_ship(self, create: CreateReply, ship: &ShipData, base_ship: Option<&ShipData>) -> CreateReply {
        let base_ship = base_ship.unwrap_or(ship);

        let mut embed = CreateEmbed::new()
            .author(super::get_ship_wiki_url(base_ship))
            .color(ship.rarity.color_rgb());

        for mount in &ship.shadow_equip {
            if mount.weapons.is_empty() {
                continue;
            }

            let mut value = String::new();
            for weapon in &mount.weapons {
                write!(value, "{}\n\n", super::fmt_shared::WeaponFormat::new(weapon)).discard();
            }

            embed = embed.field(
                format!("**`{: >3.0}%`** {}", mount.efficiency * 100f64, mount.name),
                value,
                true
            );
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

impl ButtonArgsModify for ViewShadowEquip {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        let ship = data.azur_lane().ship_by_id(self.inner.ship_id).ok_or(ShipParseError)?;
        Ok(match self.inner.retrofit.and_then(|index| ship.retrofits.get(usize::from(index))) {
            None => self.modify_with_ship(create, ship, None),
            Some(retrofit) => self.modify_with_ship(create, retrofit, Some(ship))
        })
    }
}