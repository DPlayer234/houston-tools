use azur_lane::equip::*;

use super::AzurParseError;
use crate::buttons::*;

/// Views an augment.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub equip_id: u32,
    mode: ButtonMessageMode,
}

impl View {
    /// Creates a new instance.
    pub fn new(equip_id: u32) -> Self {
        Self { equip_id, mode: ButtonMessageMode::Edit }
    }

    /// Makes the button send a new message.
    pub fn new_message(mut self) -> Self {
        self.mode = ButtonMessageMode::New;
        self
    }

    /// Modifies the create-reply with a preresolved equipment.
    pub fn modify_with_equip<'a>(
        mut self,
        create: CreateReply<'a>,
        equip: &'a Equip,
    ) -> CreateReply<'a> {
        self.mode = ButtonMessageMode::Edit;
        let description = format!(
            "**{}**\n{}",
            equip.kind.name(),
            crate::fmt::azur::EquipStats::new(equip)
        );

        let embed = CreateEmbed::new()
            .color(equip.rarity.color_rgb())
            .author(CreateEmbedAuthor::new(&equip.name))
            .description(description)
            .fields(equip.weapons.iter().map(|weapon| (
                weapon.kind.name(),
                crate::fmt::azur::Details::new(weapon).no_kind().to_string(),
                true,
            )))
            .fields(equip.skills.iter().map(|skill| (
                format!("{} {}", skill.category.emoji(), skill.name),
                utils::text::truncate(&skill.description, 1000),
                false,
            )))
            .fields(self.get_disallowed_field(equip));

        create.embed(embed).components(vec![])
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
    fn create_reply(self, ctx: ButtonContext<'_>) -> anyhow::Result<CreateReply<'_>> {
        let equip = ctx.data.azur_lane().equip_by_id(self.equip_id).ok_or(AzurParseError::Equip)?;
        Ok(self.modify_with_equip(ctx.create_reply(), equip))
    }

    fn message_mode(&self) -> ButtonMessageMode {
        self.mode
    }
}
