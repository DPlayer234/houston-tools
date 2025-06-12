use azur_lane::equip::*;
use azur_lane::skill::*;
use utils::text::{WriteStr as _, truncate};

use super::{AzurParseError, acknowledge_unloaded};
use crate::buttons::prelude::*;
use crate::config::emoji;
use crate::fmt::Join;
use crate::modules::azur::LoadedConfig;

/// Views an augment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View<'v> {
    pub augment_id: u32,
    #[serde(borrow)]
    pub back: Option<Nav<'v>>,
}

impl<'v> View<'v> {
    /// Creates a new instance.
    pub fn new(augment_id: u32) -> Self {
        Self {
            augment_id,
            back: None,
        }
    }

    /// Sets the back button target.
    pub fn back(mut self, back: Nav<'v>) -> Self {
        self.back = Some(back);
        self
    }

    /// Modifies the create-reply with a preresolved augment.
    pub fn create_with_augment<'a>(
        self,
        azur: LoadedConfig<'a>,
        augment: &'a Augment,
    ) -> CreateReply<'a> {
        let description = crate::fmt::azur::AugmentStats::new(augment).to_string();

        let embed = CreateEmbed::new()
            .author(CreateEmbedAuthor::new(&augment.name))
            .description(description)
            .color(augment.rarity.color_rgb())
            .fields(self.get_skill_field("Effect", augment.effect.as_ref()))
            .fields(self.get_skill_upgrade_field("Skill Upgrades", &augment.skill_upgrades));

        let mut components = Vec::new();

        if let Some(back) = &self.back {
            components.push(
                CreateButton::new(back.to_custom_id())
                    .emoji(emoji::back())
                    .label("Back"),
            );
        }

        if augment.effect.is_some() || !augment.skill_upgrades.is_empty() {
            let source = super::skill::ViewSource::Augment(augment.augment_id);
            let view_skill = super::skill::View::with_back(source, self.to_nav());
            components.push(CreateButton::new(view_skill.to_custom_id()).label("Effect"));
        }

        components.push(match &augment.usability {
            AugmentUsability::HullTypes(hull_types) => {
                let fmt = Join::COMMA.display_as(hull_types, |h| h.designation());
                let label = format!("For: {fmt}");

                CreateButton::new("=dummy-usability")
                    .label(truncate(label, 80))
                    .disabled(true)
            },
            AugmentUsability::UniqueShipId(ship_id) => {
                if let Some(ship) = azur.game_data().ship_by_id(*ship_id) {
                    let view = super::ship::View::new(ship.group_id).back(self.to_nav());
                    let label = format!("For: {}", ship.name);
                    CreateButton::new(view.to_custom_id()).label(truncate(label, 80))
                } else {
                    CreateButton::new("=dummy-usability")
                        .label("<Invalid>")
                        .disabled(true)
                }
            },
        });

        CreateReply::new()
            .embed(embed)
            .components(vec![CreateActionRow::buttons(components)])
    }

    /// Creates the field for a skill summary.
    fn get_skill_field<'a>(
        &self,
        label: &'a str,
        skill: Option<&Skill>,
    ) -> Option<EmbedFieldCreate<'a>> {
        skill.map(|s| {
            embed_field_create(
                label,
                format!("{} **{}**", s.category.emoji(), s.name),
                false,
            )
        })
    }

    /// Creates the field for the skill upgrades' summary.
    fn get_skill_upgrade_field<'a>(
        &self,
        label: &'a str,
        skills: &[AugmentSkillUpgrade],
    ) -> Option<EmbedFieldCreate<'a>> {
        (!skills.is_empty()).then(|| {
            let mut text = String::new();
            for s in skills {
                if !text.is_empty() {
                    text.push('\n');
                }

                write!(text, "{} **{}**", s.skill.category.emoji(), s.skill.name);
            }

            embed_field_create(label, text, false)
        })
    }
}

button_value!(for<'v> View<'v>, 2);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        acknowledge_unloaded(&ctx).await?;

        let azur = ctx.data.config().azur()?;
        let augment = azur
            .game_data()
            .augment_by_id(self.augment_id)
            .ok_or(AzurParseError::Augment)?;

        let create = self.create_with_augment(azur, augment);
        ctx.edit(create.into()).await
    }
}
