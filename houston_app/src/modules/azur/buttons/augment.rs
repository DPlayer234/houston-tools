use azur_lane::equip::*;
use azur_lane::skill::*;

use super::AzurParseError;
use crate::buttons::prelude::*;

/// Views an augment.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub augment_id: u32,
    pub back: Option<CustomData>,
}

impl View {
    /// Creates a new instance.
    pub fn new(augment_id: u32) -> Self {
        Self { augment_id, back: None }
    }

    /// Sets the back button target.
    pub fn back(mut self, back: CustomData) -> Self {
        self.back = Some(back);
        self
    }

    /// Modifies the create-reply with a preresolved augment.
    pub fn create_with_augment<'a>(
        self,
        data: &'a HBotData,
        augment: &'a Augment,
    ) -> CreateReply<'a> {
        let description = crate::fmt::azur::AugmentStats::new(augment).to_string();

        let embed = CreateEmbed::new()
            .author(CreateEmbedAuthor::new(&augment.name))
            .description(description)
            .color(augment.rarity.color_rgb())
            .fields(self.get_skill_field("Effect", augment.effect.as_ref()))
            .fields(self.get_skill_field("Skill Upgrade", augment.skill_upgrade.as_ref().map(|s| &s.skill)));

        let mut components = Vec::new();

        if let Some(back) = &self.back {
            components.push(CreateButton::new(back.to_custom_id()).emoji('⏪').label("Back"));
        }

        if augment.effect.is_some() || augment.skill_upgrade.is_some() {
            let source = super::skill::ViewSource::Augment(augment.augment_id);
            let view_skill = super::skill::View::with_back(source, self.to_custom_data());
            components.push(CreateButton::new(view_skill.to_custom_id()).label("Effect"));
        }

        components.push(match &augment.usability {
            AugmentUsability::HullTypes(hull_types) => {
                let mut label = "For: ".to_owned();
                crate::fmt::write_join(&mut label, hull_types.iter().map(|h| h.designation()), ", ")
                    .expect("writing to String cannot fail");

                let label = utils::text::truncate(label, 25);
                CreateButton::new("=dummy-usability").label(label).disabled(true)
            },
            AugmentUsability::UniqueShipId(ship_id) => if let Some(ship) = data.azur_lane().ship_by_id(*ship_id) {
                let view = super::ship::View::new(ship.group_id).back(self.to_custom_data());
                let label = utils::text::truncate(format!("For: {}", ship.name), 25);
                CreateButton::new(view.to_custom_id()).label(label)
            } else {
                CreateButton::new("=dummy-usability").label("<Invalid>").disabled(true)
            },
        });

        CreateReply::new().embed(embed).components(vec![CreateActionRow::buttons(components)])
    }

    /// Creates the field for a skill summary.
    fn get_skill_field<'a>(&self, label: &'a str, skill: Option<&Skill>) -> Option<SimpleEmbedFieldCreate<'a>> {
        skill.map(|s| {
            (label, format!("{} **{}**", s.category.emoji(), s.name), false)
        })
    }
}

impl ButtonMessage for View {
    fn create_reply(self, ctx: ButtonContext<'_>) -> anyhow::Result<CreateReply<'_>> {
        let augment = ctx.data.azur_lane().augment_by_id(self.augment_id).ok_or(AzurParseError::Augment)?;
        Ok(self.create_with_augment(ctx.data, augment))
    }
}
