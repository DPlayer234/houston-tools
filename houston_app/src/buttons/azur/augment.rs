use std::fmt::Write;

use azur_lane::equip::*;
use azur_lane::ship::*;
use azur_lane::skill::*;
use utils::Discard;

use crate::buttons::*;
use super::AugmentParseError;

/// Views an augment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub augment_id: u32,
    pub back: Option<CustomData>
}

impl View {
    /// Creates a new instance.
    #[allow(dead_code)] // planned for future use
    pub fn new(augment_id: u32) -> Self {
        Self { augment_id, back: None }
    }

    /// Creates a new instance including a button to go back with some custom ID.
    pub fn with_back(augment_id: u32, back: CustomData) -> Self {
        Self { augment_id, back: Some(back) }
    }

    /// Modifies the create-reply with a preresolved augment.
    pub fn modify_with_augment(self, create: CreateReply, augment: &Augment) -> CreateReply {
        let mut description = String::new();
        for chunk in augment.stat_bonuses.chunks(3) {
            if !description.is_empty() { description.push('\n'); }
            for (index, stat) in chunk.iter().enumerate() {
                if index != 0 { description.push_str(" \u{2E31} "); }

                let name = stat.stat_kind.name();
                write!(description, "**`{}:`**`{: >len$}`", name, stat.amount + stat.random, len = 7 - name.len()).discard();
            }
        }

        let embed = CreateEmbed::new()
            .author(CreateEmbedAuthor::new(&augment.name))
            .description(description)
            .color(ShipRarity::SR.color_rgb())
            .fields(self.get_skill_field("Effect", augment.effect.as_ref()))
            .fields(self.get_skill_field("Skill Upgrade", augment.skill_upgrade.as_ref()));

        let mut components = Vec::new();

        if augment.effect.is_some() || augment.skill_upgrade.is_some() {
            let source = super::skill::ViewSource::Augment(augment.augment_id);
            let view_skill = super::skill::View::with_back(source, self.to_custom_data());
            components.push(CreateButton::new(view_skill.to_custom_id()).label("Effect"));
        }

        if let Some(back) = self.back {
            components.insert(0, CreateButton::new(back.to_custom_id()).emoji('⏪').label("Back"));
        }

        create.embed(embed).components(vec![CreateActionRow::Buttons(components)])
    }

    /// Creates the field for a skill summary.
    fn get_skill_field(&self, label: &'static str, skill: Option<&Skill>) -> Option<SimpleEmbedFieldCreate> {
        skill.map(|s| {
            (label, format!("{} **{}**", s.category.emoji(), s.name), false)
        })
    }
}

impl ButtonMessage for View {
    fn create_reply(self, ctx: ButtonContext<'_>) -> anyhow::Result<CreateReply> {
        let augment = ctx.data.azur_lane().augment_by_id(self.augment_id).ok_or(AugmentParseError)?;
        Ok(self.modify_with_augment(ctx.create_reply(), augment))
    }
}
