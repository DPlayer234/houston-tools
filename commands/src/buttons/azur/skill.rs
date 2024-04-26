//use crate::internal::prelude::*;
use crate::buttons::*;
use azur_lane::equip::*;
use azur_lane::ship::*;
use azur_lane::skill::*;

use super::AugmentParseError;
use super::ShipParseError;

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct ViewSkill {
    pub source: ViewSkillSource,
    pub skill_index: Option<u8>,
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

type OwnedCreateEmbedField = (String, String, bool);

impl ViewSkill {
    pub fn new(source: ViewSkillSource) -> Self {
        Self { source, skill_index: None, back: None }
    }

    pub fn with_back(source: ViewSkillSource, back: String) -> Self {
        Self { source, skill_index: None, back: Some(back) }
    }

    pub fn modify_with_skills<'a>(self, create: CreateReply, iterator: impl Iterator<Item = &'a Skill>) -> anyhow::Result<CreateReply> {
        let index = self.skill_index.map(usize::from);
        let mut embed = CreateEmbed::new();
        let mut components = Vec::new();

        if let Some(ref back) = self.back {
            components.push(CreateButton::new(back.as_str()).emoji('⏪').label("Back"));
        }

        for (t_index, skill) in iterator.enumerate().take(4) {
            if Some(t_index) == index {
                embed = embed.color(skill.category.data().color_rgb);
                embed = embed.fields(self.create_ex_skill_field(skill));
            } else {
                embed = embed.fields(self.create_skill_field(skill));
            }

            if !skill.barrages.is_empty() {
                let button = self.new_button(utils::field!(Self: skill_index), Some(t_index as u8), || Sentinel::new(1, t_index as u32))
                    .label(skill.name.as_ref())
                    .style(ButtonStyle::Secondary);
    
                components.push(button);
            }
        }

        let rows = vec![
            CreateActionRow::Buttons(components)
        ];

        Ok(create.embed(embed).components(rows))
    }

    pub fn modify_with_ship(self, create: CreateReply, ship: &ShipData) -> anyhow::Result<CreateReply> {
        self.modify_with_skills(create, ship.skills.iter())
    }

    pub fn modify_with_augment(self, create: CreateReply, augment: &Augment) -> anyhow::Result<CreateReply> {
        self.modify_with_skills(create, augment.effect.iter().chain(augment.skill_upgrade.as_ref()))
    }

    fn create_skill_field(&self, skill: &Skill) -> [OwnedCreateEmbedField; 1] {
        [(
            format!("{} {}", skill.category.data().emoji, skill.name),
            skill.description.as_ref().to_owned(),
            false
        )]
    }

    fn create_ex_skill_field(&self, skill: &Skill) -> [OwnedCreateEmbedField; 1] {
        [(
            format!("{} __{}__", skill.category.data().emoji, skill.name),
            skill.description.as_ref().to_owned(),
            false
        )]
    }
}

impl ButtonArgsModify for ViewSkill {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        match self.source {
            ViewSkillSource::Ship(ship_id) => {
                let ship = data.azur_lane.ship_by_id(ship_id).ok_or(ShipParseError)?;
                self.modify_with_ship(create, ship)
            }
            ViewSkillSource::Augment(augment_id) => {
                let augment = data.azur_lane.augment_by_id(augment_id).ok_or(AugmentParseError)?;
                self.modify_with_augment(create, augment)
            }
        }
    }
}
