use std::fmt::Write;

use azur_lane::equip::*;
use utils::Discard;

use crate::buttons::*;
use super::EquipParseError;

/// Views an augment.
#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct ViewEquip {
    pub equip_id: u32,
}

impl From<ViewEquip> for ButtonArgs {
    fn from(value: ViewEquip) -> Self {
        ButtonArgs::ViewEquip(value)
    }
}

impl ViewEquip {
    /// Creates a new instance.
    pub fn new(equip_id: u32) -> Self {
        Self { equip_id }
    }

    /// Modifies the create-reply with a preresolved equipment.
    pub fn modify_with_equip(self, create: CreateReply, equip: &Equip) -> CreateReply {
        let mut description = String::new();
        for chunk in equip.stat_bonuses.chunks(3) {
            if !description.is_empty() { description.push('\n'); }
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
                format!("{:#}", super::fmt_shared::WeaponFormat::new(weapon)),
                true,
            )))
            .fields(equip.skills.iter().map(|skill| (
                format!("{} {}", skill.category.emoji(), skill.name),
                utils::text::truncate(&skill.description, 1000),
                false,
            )));

        create.embed(embed)
    }
}

impl ButtonArgsModify for ViewEquip {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        let equip = data.azur_lane().equip_by_id(self.equip_id).ok_or(EquipParseError)?;
        Ok(self.modify_with_equip(create, equip))
    }
}
