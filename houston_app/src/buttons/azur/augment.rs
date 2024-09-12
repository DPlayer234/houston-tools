use azur_lane::equip::*;
use azur_lane::skill::*;
use utils::Discard;

use crate::buttons::*;
use super::AugmentParseError;

/// Views an augment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub augment_id: u32,
    mode: ButtonMessageMode,
}

impl View {
    /// Creates a new instance.
    pub fn new(augment_id: u32) -> Self {
        Self { augment_id, mode: ButtonMessageMode::Edit }
    }

    /// Makes the button send a new message.
    pub fn new_message(mut self) -> Self {
        self.mode = ButtonMessageMode::New;
        self
    }

    /// Modifies the create-reply with a preresolved augment.
    pub fn modify_with_augment(mut self, data: &HBotData, create: CreateReply, augment: &Augment) -> CreateReply {
        self.mode = ButtonMessageMode::Edit;
        let description = format!("{}", crate::fmt::azur::AugmentStats::new(augment));

        let embed = CreateEmbed::new()
            .author(CreateEmbedAuthor::new(&augment.name))
            .description(description)
            .color(augment.rarity.color_rgb())
            .fields(self.get_skill_field("Effect", augment.effect.as_ref()))
            .fields(self.get_skill_field("Skill Upgrade", augment.skill_upgrade.as_ref().map(|s| &s.skill)));

        let mut components = Vec::new();

        if augment.effect.is_some() || augment.skill_upgrade.is_some() {
            let source = super::skill::ViewSource::Augment(augment.augment_id);
            let view_skill = super::skill::View::with_back(source, self.to_custom_data());
            components.push(CreateButton::new(view_skill.to_custom_id()).label("Effect"));
        }

        components.push(match &augment.usability {
            AugmentUsability::HullTypes(hull_types) => {
                let mut label = "For: ".to_owned();
                crate::fmt::write_join(&mut label, hull_types.iter().map(|h| h.designation()), ", ").discard();
                let label = utils::text::truncate(label, 25);
                CreateButton::new("=dummy-usability").label(label).disabled(true)
            },
            AugmentUsability::UniqueShipId(ship_id) => if let Some(ship) = data.azur_lane().ship_by_id(*ship_id) {
                let view = super::ship::View::new(ship.group_id).new_message();
                let label = utils::text::truncate(format!("For: {}", ship.name), 25);
                CreateButton::new(view.to_custom_id()).label(label)
            } else {
                CreateButton::new("=dummy-usability").label("<Invalid>").disabled(true)
            },
        });

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
        Ok(self.modify_with_augment(ctx.data, ctx.create_reply(), augment))
    }

    fn message_mode(&self) -> ButtonMessageMode {
        self.mode
    }
}
