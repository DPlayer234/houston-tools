use azur_lane::equip::*;
use utils::text::{WriteStr as _, truncate};

use super::AzurParseError;
use crate::buttons::prelude::*;
use crate::config::emoji;
use crate::fmt::Join;
use crate::modules::azur::LoadedConfig;

/// Views an augment.
#[derive(Debug, Clone, Serialize, Deserialize, ConstBuilder)]
pub struct View<'v> {
    augment_id: u32,
    #[serde(borrow)]
    #[builder(default = None, setter(strip_option))]
    back: Option<Nav<'v>>,
}

impl View<'_> {
    /// Modifies the create-reply with a preresolved augment.
    pub fn create_with_augment<'a>(
        self,
        azur: LoadedConfig<'a>,
        augment: &'a Augment,
    ) -> CreateReply<'a> {
        let mut components = CreateComponents::new();

        components.push(CreateTextDisplay::new(format!("### {}", augment.name)));
        components.push(CreateSeparator::new(true));
        components.push(CreateTextDisplay::new(
            crate::fmt::azur::AugmentStats::new(augment).to_string(),
        ));

        if let Some(effect) = &augment.effect {
            components.push(CreateSeparator::new(true));
            components.push(CreateTextDisplay::new(format!(
                "### Effect\n{} **{}**",
                effect.category.emoji(),
                effect.name
            )));
        }

        if !augment.skill_upgrades.is_empty() {
            let mut text = String::new();
            text.push_str("### Skill Upgrades\n");

            for s in &augment.skill_upgrades {
                writeln!(text, "{} **{}**", s.skill.category.emoji(), s.skill.name);
            }

            components.push(CreateSeparator::new(true));
            components.push(CreateTextDisplay::new(text));
        }

        let mut nav = Vec::new();

        if let Some(back) = &self.back {
            let button = CreateButton::new(back.to_custom_id())
                .emoji(emoji::back())
                .label("Back");

            nav.push(button);
        }

        if augment.effect.is_some() || !augment.skill_upgrades.is_empty() {
            let view_skill = super::skill::View::builder()
                .augment_source(augment.augment_id)
                .back(self.to_nav())
                .build();

            nav.push(CreateButton::new(view_skill.to_custom_id()).label("Effect"));
        }

        nav.push(match &augment.usability {
            AugmentUsability::HullTypes(hull_types) => {
                let fmt = Join::COMMA.display_as(hull_types, |h| h.designation());
                let label = format!("For: {fmt}");

                CreateButton::new("=dummy-usability")
                    .label(truncate(label, 80))
                    .disabled(true)
            },
            AugmentUsability::UniqueShipId(ship_id) => {
                if let Some(ship) = azur.game_data().ship_by_id(*ship_id) {
                    let view = super::ship::View::builder()
                        .ship_id(ship.group_id)
                        .back(self.to_nav())
                        .build();

                    let label = format!("For: {}", ship.name);
                    CreateButton::new(view.to_custom_id()).label(truncate(label, 80))
                } else {
                    CreateButton::new("=dummy-usability")
                        .label("<Invalid>")
                        .disabled(true)
                }
            },
        });

        components.push(CreateActionRow::buttons(nav));

        CreateReply::new().components_v2(components![
            CreateContainer::new(components).accent_color(augment.rarity.color_rgb())
        ])
    }
}

button_value!(for<'v> View<'v>, 2);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        let azur = ctx.data.config().azur()?;
        let augment = azur
            .game_data()
            .augment_by_id(self.augment_id)
            .ok_or(AzurParseError::Augment)?;

        let create = self.create_with_augment(azur, augment);
        ctx.edit(create.into()).await
    }
}
