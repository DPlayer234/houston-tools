use azur_lane::equip::*;
use utils::text::truncate;

use super::AzurParseError;
use crate::buttons::prelude::*;
use crate::config::emoji;
use crate::fmt::Join;

/// Views an augment.
#[derive(Debug, Clone, Serialize, Deserialize, ConstBuilder)]
pub struct View<'v> {
    pub equip_id: u32,
    #[serde(borrow)]
    #[builder(default = None, setter(strip_option))]
    pub back: Option<Nav<'v>>,
}

impl View<'_> {
    /// Modifies the create-reply with a preresolved equipment.
    pub fn create_with_equip(self, equip: &Equip) -> CreateReply<'_> {
        let mut components = CreateComponents::new();
        components.push(CreateTextDisplay::new(format!("## {}", equip.name)));
        components.push(CreateSeparator::new(true));

        components.push(CreateTextDisplay::new(format!(
            "### {}\n{}",
            equip.kind.name(),
            crate::fmt::azur::EquipStats::new(equip)
        )));

        for (index, weapon) in equip.weapons.iter().enumerate() {
            components.push(CreateSeparator::new(index == 0));
            components.push(CreateTextDisplay::new(format!(
                "### {}\n{}",
                weapon.kind.name(),
                crate::fmt::azur::Details::new(weapon).no_kind(),
            )));
        }

        for (index, skill) in equip.skills.iter().enumerate() {
            components.push(CreateSeparator::new(index == 0));
            components.push(CreateTextDisplay::new(format!(
                "### {} {}",
                skill.category.emoji(),
                skill.name
            )));
            components.push(CreateTextDisplay::new(truncate(&skill.description, 1000)));
        }

        if !equip.hull_disallowed.is_empty() {
            components.push(CreateSeparator::new(true));
            components.push(CreateTextDisplay::new(format!(
                "**Cannot be equipped by:**\n> {}",
                Join::COMMA.display_as(&equip.hull_disallowed, |h| h.designation()),
            )));
        }

        if let Some(back) = &self.back {
            let button = CreateButton::new(back.to_custom_id())
                .emoji(emoji::back())
                .label("Back");

            components.push(CreateSeparator::new(true));
            components.push(CreateActionRow::buttons(vec![button]));
        }

        CreateReply::new().components_v2(components![
            CreateContainer::new(components).accent_color(equip.rarity.color_rgb())
        ])
    }
}

button_value!(for<'v> View<'v>, 7);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        let azur = ctx.data.config().azur()?;
        let equip = azur
            .game_data()
            .equip_by_id(self.equip_id)
            .ok_or(AzurParseError::Equip)?;

        let create = self.create_with_equip(equip);
        ctx.edit(create.into()).await
    }
}
