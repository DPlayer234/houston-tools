use std::fmt::Write;

use azur_lane::equip::*;
use utils::Discard;

use crate::buttons::*;
use super::EquipParseError;

/// Views an augment.
#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct View {
    pub equip_id: u32,
}

impl View {
    /// Creates a new instance.
    pub fn new(equip_id: u32) -> Self {
        Self { equip_id }
    }

    /// Modifies the create-reply with a preresolved equipment.
    pub fn modify_with_equip(self, create: CreateReply, equip: &Equip) -> CreateReply {
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
                crate::fmt::azur::WeaponFormat::new(weapon).to_string_no_kind(),
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

impl ButtonArgsModify for View {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        let equip = data.azur_lane().equip_by_id(self.equip_id).ok_or(EquipParseError)?;
        Ok(self.modify_with_equip(create, equip))
    }
}
