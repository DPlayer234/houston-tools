use azur_lane::equip::*;
use azur_lane::skill::Skill;
use utils::text::{WriteStr as _, truncate};

use super::AzurParseError;
use crate::buttons::prelude::*;
use crate::config::emoji;
use crate::fmt::Join;

/// Views an augment.
#[derive(Debug, Clone, Serialize, Deserialize, ConstBuilder)]
pub struct View<'v> {
    equip_id: u32,
    #[serde(borrow)]
    #[builder(default = None, setter(strip_option))]
    back: Option<Nav<'v>>,
}

impl View<'_> {
    /// Modifies the create-reply with a preresolved equipment.
    pub fn create_with_equip(self, equip: &Equip) -> CreateReply<'_> {
        let mut components = CreateComponents::new();
        components.push(CreateTextDisplay::new(format!("## {}", equip.name)));
        components.push(CreateSeparator::new(true));

        components.push(CreateTextDisplay::new(format!(
            "### {}\n{}",
            equip.kind.name(),
            crate::fmt::azur::EquipStats::new(equip)
        )));

        if !equip.weapons.is_empty() {
            components.push(CreateSeparator::new(true));
        }

        for weapon in &equip.weapons {
            components.push(CreateTextDisplay::new(format!(
                "### {}\n{}",
                weapon.kind.name(),
                crate::fmt::azur::Details::new(weapon).no_kind(),
            )));
        }

        match inline_skill(equip) {
            InlineSkill::None => { /* nothing to display */ },
            InlineSkill::Yes(skill) => {
                components.push(CreateSeparator::new(true));
                components.push(CreateTextDisplay::new(format!(
                    "### {} {}",
                    skill.category.emoji(),
                    skill.name
                )));
                components.push(CreateTextDisplay::new(truncate(&skill.description, 1000)));
            },
            InlineSkill::No(skills) => {
                components.push(CreateSeparator::new(true));
                components.push(self.get_skills_field(skills));
            },
        }

        if !equip.hull_disallowed.is_empty() {
            components.push(CreateSeparator::new(true));
            components.push(CreateTextDisplay::new(format!(
                "**Cannot be equipped by:**\n> {}",
                Join::COMMA.display_as(&equip.hull_disallowed, |h| h.designation()),
            )));
        }

        if let Some(back) = &self.back {
            let button = CreateButton::new(back.to_custom_id())
                .emoji(emoji::back())
                .label("Back");

            components.push(CreateSeparator::new(true));
            components.push(CreateActionRow::buttons(vec![button]));
        }

        CreateReply::new().components_v2(components![
            CreateContainer::new(components).accent_color(equip.rarity.color_rgb())
        ])
    }

    fn get_skills_field<'a>(&self, skills: &[Skill]) -> CreateComponent<'a> {
        let mut text = String::new();
        text.push_str("### Skills\n");

        for s in skills {
            writeln!(text, "{} **{}**", s.category.emoji(), s.name);
        }

        let button = {
            use super::skill::{View, ViewSource};

            let view_skill = View::builder()
                .source(ViewSource::Equip(self.equip_id))
                .back(self.to_nav())
                .build();

            CreateButton::new(view_skill.to_custom_id())
                .label("Info")
                .style(ButtonStyle::Secondary)
        };

        CreateSection::new(
            section_components![CreateTextDisplay::new(text)],
            CreateSectionAccessory::Button(button),
        )
        .into_component()
    }
}

enum InlineSkill<'a> {
    None,
    Yes(&'a Skill),
    No(&'a [Skill]),
}

fn inline_skill(equip: &Equip) -> InlineSkill<'_> {
    match equip.skills.as_slice() {
        [] => InlineSkill::None,
        [skill] if skill.barrages.is_empty() && skill.new_weapons.is_empty() => {
            InlineSkill::Yes(skill)
        },
        skills => InlineSkill::No(skills),
    }
}

button_value!(for<'v> View<'v>, 7);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        let data = ctx.data_ref();
        let azur = data.config().azur()?;
        let equip = azur
            .game_data()
            .equip_by_id(self.equip_id)
            .ok_or(AzurParseError::Equip)?;

        let create = self.create_with_equip(equip);
        ctx.edit(create.into()).await
    }
}
