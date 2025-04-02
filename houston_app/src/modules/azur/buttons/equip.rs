use azur_lane::equip::*;
use utils::text::truncate;

use super::{AzurParseError, acknowledge_unloaded};
use crate::buttons::prelude::*;
use crate::config::emoji;
use crate::fmt::Join;

/// Views an augment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View<'v> {
    pub equip_id: u32,
    #[serde(borrow)]
    pub back: Option<CustomData<'v>>,
}

impl<'v> View<'v> {
    /// Creates a new instance.
    pub fn new(equip_id: u32) -> Self {
        Self {
            equip_id,
            back: None,
        }
    }

    /// Sets the back button target.
    pub fn back(mut self, back: CustomData<'v>) -> Self {
        self.back = Some(back);
        self
    }

    /// Modifies the create-reply with a preresolved equipment.
    pub fn create_with_equip(self, equip: &Equip) -> CreateReply<'_> {
        let description = format!(
            "**{}**\n{}",
            equip.kind.name(),
            crate::fmt::azur::EquipStats::new(equip)
        );

        let embed = CreateEmbed::new()
            .color(equip.rarity.color_rgb())
            .author(CreateEmbedAuthor::new(&equip.name))
            .description(description)
            .fields(equip.weapons.iter().map(|weapon| {
                (
                    weapon.kind.name(),
                    crate::fmt::azur::Details::new(weapon).no_kind().to_string(),
                    true,
                )
            }))
            .fields(equip.skills.iter().map(|skill| {
                (
                    format!("{} {}", skill.category.emoji(), skill.name),
                    truncate(&skill.description, 1000),
                    false,
                )
            }))
            .fields(self.get_disallowed_field(equip));

        let components = match &self.back {
            Some(back) => {
                let button = CreateButton::new(back.to_custom_id())
                    .emoji(emoji::back())
                    .label("Back");
                vec![CreateActionRow::buttons(vec![button])]
            },
            None => vec![],
        };

        CreateReply::new().embed(embed).components(components)
    }

    fn get_disallowed_field<'a>(&self, equip: &Equip) -> Option<EmbedFieldCreate<'a>> {
        (!equip.hull_disallowed.is_empty()).then(|| {
            let fmt = Join::COMMA.display_as(&equip.hull_disallowed, |h| h.designation());
            let text = format!("> {fmt}");

            embed_field_create("Cannot be equipped by:", text, false)
        })
    }
}

impl ButtonArgsReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        acknowledge_unloaded(&ctx).await?;

        let azur = ctx.data.config().azur()?;
        let equip = azur
            .game_data()
            .equip_by_id(self.equip_id)
            .ok_or(AzurParseError::Equip)?;

        let create = self.create_with_equip(equip);
        ctx.edit(create.into()).await
    }
}
