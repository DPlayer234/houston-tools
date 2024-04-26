//use crate::internal::prelude::*;
use crate::buttons::*;
use azur_lane::skill::*;

use super::AugmentParseError;
use super::ShipParseError;
use super::SkillParseError;

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct ViewSkill {
    pub source: ViewSkillSource,
    pub skill_index: u8,
    pub back: Option<String>
}

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub enum ViewSkillSource {
    Ship(u32),
    Augment(u32),
}

impl From<ViewSkill> for ButtonArgs {
    fn from(value: ViewSkill) -> Self {
        ButtonArgs::ViewSkill(value)
    }
}

impl ViewSkill {
    pub fn new(source: ViewSkillSource, skill_index: u8) -> Self {
        Self { source, skill_index, back: None }
    }

    pub fn with_back(source: ViewSkillSource, skill_index: u8, back: String) -> Self {
        Self { source, skill_index, back: Some(back) }
    }

    pub fn modify_with_skill(self, create: CreateReply, skill: &Skill) -> anyhow::Result<CreateReply> {
        let embed = CreateEmbed::new()
            .author(CreateEmbedAuthor::new(skill.name.as_ref()))
            .description(skill.description.as_ref())
            .color(skill.category.data().color_rgb);
        
        let components = self.back
            .map(|back| CreateButton::new(back).emoji('⏪').label("Back"))
            .into_iter()
            .collect();

        Ok(create.embed(embed).components(vec![CreateActionRow::Buttons(components)]))
    }
}

impl ButtonArgsModify for ViewSkill {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        let skill = match self.source {
            ViewSkillSource::Ship(ship_id) => {
                let ship = data.azur_lane.ship_by_id(ship_id).ok_or(ShipParseError)?;
                ship.skills.get(usize::from(self.skill_index)).ok_or(SkillParseError)?
            }
            ViewSkillSource::Augment(augment_id) => {
                let augment = data.azur_lane.augment_by_id(augment_id).ok_or(AugmentParseError)?;
                match self.skill_index {
                    0 => augment.effect.as_ref().ok_or(SkillParseError)?,
                    1 => augment.skill_upgrade.as_ref().ok_or(SkillParseError)?,
                    _ => Err(SkillParseError)?
                }
            }
        };

        self.modify_with_skill(create, skill)
    }
}
