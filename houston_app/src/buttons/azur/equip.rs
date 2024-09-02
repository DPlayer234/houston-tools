use std::fmt::Write;

use azur_lane::equip::*;
use utils::Discard;

use crate::buttons::*;
use super::EquipParseError;

/// Views an augment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    pub fn modify_with_equip(mut self, create: CreateReply, equip: &Equip) -> CreateReply {
        self.mode = ButtonMessageMode::Edit;
        let mut description = format!("**{}**", equip.kind.name());

        for chunk in equip.stat_bonuses.chunks(3) {
            description.push('\n');
            for (index, stat) in chunk.iter().enumerate() {
                if index != 0 { description.push_str(" \u{2E31} "); }

                let name = stat.stat_kind.name();
                write!(description, "**`{}:`**`{: >len$}`", name, stat.amount, len = 7 - name.len()).discard();
            }
        }

        let embed = CreateEmbed::new()
            .color(equip.rarity.color_rgb())
            .author(CreateEmbedAuthor::new(&equip.name))
            .description(description)
            .fields(equip.weapons.iter().map(|weapon| (
                weapon.kind.name(),
                crate::fmt::azur::DisplayWeapon::new(weapon).no_kind().to_string(),
                true,
            )))
            .fields(equip.skills.iter().map(|skill| (
                format!("{} {}", skill.category.emoji(), skill.name),
                utils::text::truncate(&skill.description, 1000),
                false,
            )));

        create.embed(embed).components(vec![])
    }
}

impl ButtonMessage for View {
    fn create_reply(self, ctx: ButtonContext<'_>) -> anyhow::Result<CreateReply> {
        let equip = ctx.data.azur_lane().equip_by_id(self.equip_id).ok_or(EquipParseError)?;
        Ok(self.modify_with_equip(ctx.create_reply(), equip))
    }

    fn message_mode(&self) -> ButtonMessageMode {
        self.mode
    }
}
