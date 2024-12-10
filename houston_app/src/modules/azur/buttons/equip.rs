use azur_lane::equip::*;

use super::AzurParseError;
use crate::buttons::prelude::*;

/// Views an augment.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub equip_id: u32,
    pub back: Option<CustomData>,
}

impl View {
    /// Creates a new instance.
    pub fn new(equip_id: u32) -> Self {
        Self {
            equip_id,
            back: None,
        }
    }

    /// Sets the back button target.
    pub fn back(mut self, back: CustomData) -> Self {
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
                    utils::text::truncate(&skill.description, 1000),
                    false,
                )
            }))
            .fields(self.get_disallowed_field(equip));

        let components = match &self.back {
            Some(back) => {
                let button = CreateButton::new(back.to_custom_id())
                    .emoji('⏪')
                    .label("Back");
                vec![CreateActionRow::buttons(vec![button])]
            },
            None => vec![],
        };

        CreateReply::new().embed(embed).components(components)
    }

    fn get_disallowed_field<'a>(&self, equip: &Equip) -> Option<SimpleEmbedFieldCreate<'a>> {
        (!equip.hull_disallowed.is_empty()).then(|| {
            let mut text = "> ".to_owned();
            let designations = equip.hull_disallowed.iter().map(|h| h.designation());

            crate::fmt::write_join(&mut text, designations, ", ")
                .expect("writing to String cannot fail");

            ("Cannot be equipped by:", text, false)
        })
    }
}

impl ButtonMessage for View {
    fn edit_reply(self, ctx: ButtonContext<'_>) -> Result<EditReply<'_>> {
        let equip = ctx
            .data
            .azur_lane()
            .equip_by_id(self.equip_id)
            .ok_or(AzurParseError::Equip)?;
        Ok(self.create_with_equip(equip).into())
    }
}
