use std::fmt::Write;
//use crate::internal::prelude::*;
use crate::buttons::*;
use azur_lane::equip::*;
use azur_lane::ship::*;
use azur_lane::skill::*;
use utils::Discard;

use super::AugmentParseError;

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct ViewAugment {
    pub augment_id: u32,
    pub back: Option<String>
}

impl From<ViewAugment> for ButtonArgs {
    fn from(value: ViewAugment) -> Self {
        ButtonArgs::ViewAugment(value)
    }
}

impl ViewAugment {
    pub fn new(augment_id: u32) -> Self {
        Self { augment_id, back: None }
    }

    pub fn with_back(augment_id: u32, back: String) -> Self {
        Self { augment_id, back: Some(back) }
    }

    pub fn modify_with_augment(self, create: CreateReply, augment: &Augment) -> anyhow::Result<CreateReply> {
        let mut description = String::new();
        for chunk in augment.stat_bonuses.chunks(3) {
            if !description.is_empty() { description.push('\n'); }
            for (index, stat) in chunk.iter().enumerate() {
                if index != 0 { description.push_str(" \u{2E31} "); }

                let name = stat.stat_kind.data().name;
                write!(description, "**`{}:`**`{: >len$}`", name, stat.amount + stat.random, len = 7 - name.len()).discard();
            }
        }

        let embed = CreateEmbed::new()
            .author(CreateEmbedAuthor::new(augment.name.as_ref()))
            .description(description)
            .color(ShipRarity::SR.data().color_rgb)
            .fields(self.get_skill_field("Effect", augment.effect.as_ref()))
            .fields(self.get_skill_field("Skill Upgrade", augment.skill_upgrade.as_ref()));

        let mut components = Vec::new();

        if augment.effect.is_some() || augment.skill_upgrade.is_some() {
            let source = super::skill::ViewSkillSource::Augment(augment.augment_id);
            let view_skill = super::skill::ViewSkill::with_back(source, self.clone().to_custom_id());
            components.push(CreateButton::new(view_skill.to_custom_id()).label("Effect").style(ButtonStyle::Secondary));
        }

        if let Some(back) = self.back {
            components.insert(0, CreateButton::new(back).emoji('⏪').label("Back"));
        }

        Ok(create.embed(embed).components(vec![CreateActionRow::Buttons(components)]))
    }

    fn get_skill_field(&self, label: &'static str, skill: Option<&Skill>) -> Option<SimpleEmbedFieldCreate> {
        skill.map(|s| {
            (label, format!("{} **{}**", s.category.data().emoji, s.name), false)
        })
    }
}

impl ButtonArgsModify for ViewAugment {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        let augment = data.azur_lane.augment_by_id(self.augment_id).ok_or(AugmentParseError)?;
        self.modify_with_augment(create, augment)
    }
}
