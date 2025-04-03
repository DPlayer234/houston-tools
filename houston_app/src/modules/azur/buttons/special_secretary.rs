use azur_lane::secretary::*;
use utils::text::write_str::*;

use super::{AzurParseError, acknowledge_unloaded};
use crate::buttons::prelude::*;
use crate::config::emoji;

/// Views ship lines.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View<'v> {
    pub secretary_id: u32,
    pub part: ViewPart,
    #[serde(borrow)]
    back: Option<Nav<'v>>,
}

/// Which part of the lines to display.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ViewPart {
    Main1,
    Main2,
    Holidays,
    Chime1,
    Chime2,
}

impl<'v> View<'v> {
    /// Creates a new instance.
    pub fn new(secretary_id: u32) -> Self {
        Self {
            secretary_id,
            part: ViewPart::Main1,
            back: None,
        }
    }

    pub fn back(mut self, back: Nav<'v>) -> Self {
        self.back = Some(back);
        self
    }

    pub fn create_with_sectary<'a>(
        mut self,
        data: &'a HBotData,
        secretary: &'a SpecialSecretary,
    ) -> CreateReply<'a> {
        let embed = CreateEmbed::new()
            .color(data.config().embed_color)
            .author(CreateEmbedAuthor::new(&secretary.name))
            .description(self.part.get_description(secretary));

        let mut components = Vec::new();

        let mut top_row = Vec::new();
        if let Some(back) = &self.back {
            let button = CreateButton::new(back.to_custom_id())
                .emoji(emoji::back())
                .label("Back");
            top_row.push(button);
        }

        if !top_row.is_empty() {
            components.push(CreateActionRow::buttons(top_row));
        }

        components.push(CreateActionRow::buttons(vec![
            self.button_with_part(ViewPart::Main1, secretary, "1", "< 1 >"),
            self.button_with_part(ViewPart::Main2, secretary, "2", "< 2 >"),
            self.button_with_part(ViewPart::Holidays, secretary, "3", "< 3 >"),
            self.button_with_part(ViewPart::Chime1, secretary, "4", "< 4 >"),
            self.button_with_part(ViewPart::Chime2, secretary, "5", "< 5 >"),
        ]));

        CreateReply::new().embed(embed).components(components)
    }

    /// Creates a button that redirects to a different viewed part.
    fn button_with_part<'a>(
        &mut self,
        part: ViewPart,
        secretary: &SpecialSecretary,
        label: &'a str,
        active_label: &'a str,
    ) -> CreateButton<'a> {
        let button = self
            .new_button(|s| &mut s.part, part, |u| u as u16)
            .style(ButtonStyle::Secondary)
            .label(label);
        if !part.has_texts(secretary) {
            button.disabled(true)
        } else if self.part == part {
            button.label(active_label)
        } else {
            button
        }
    }
}

/// Higher-order macro to share code logic for [`ViewPart`] functions.
macro_rules! impl_view_part_fn {
    ($self:expr, $words:expr, $add:ident) => {
        match $self {
            ViewPart::Main1 => {
                $add!("Login", login);

                for line in &$words.main_screen {
                    $add!(main line);
                }

                $add!("Touch", touch);
            }
            ViewPart::Main2 => {
                $add!("Mission Reminder", mission_reminder);
                $add!("Mission Complete", mission_complete);
                $add!("Mail Reminder", mail_reminder);
                $add!("Return to Port", return_to_port);
                $add!("Commission Complete", commission_complete);
            }
            ViewPart::Holidays => {
                $add!("Christmas", christmas);
                $add!("New Year's Eve", new_years_eve);
                $add!("New Year's Day", new_years_day);
                $add!("Valentine's Day", valentines);
                $add!("Mid-Autumn Festival", mid_autumn_festival);
                $add!("Halloween", halloween);
                $add!("Event Reminder", event_reminder);
                $add!("Change Module", change_module);
            }
            ViewPart::Chime1 => {
                if let Some(chime) = &$words.chime {
                    for (index, opt) in chime.iter().enumerate().take(12) {
                        $add!(chime index, opt);
                    }
                }
            }
            ViewPart::Chime2 => {
                if let Some(chime) = &$words.chime {
                    for (index, opt) in chime.iter().enumerate().skip(12) {
                        $add!(chime index, opt);
                    }
                }
            }
        }
    };
}

impl ViewPart {
    /// Creates the embed description for the current state.
    fn get_description(self, words: &SpecialSecretary) -> String {
        use crate::fmt::discord::escape_markdown;

        let mut result = String::new();

        macro_rules! add {
            ($label:literal, $key:ident) => {
                if let Some(text) = &words.$key {
                    write_str!(
                        result,
                        concat!("- **", $label, ":** {}\n"),
                        escape_markdown(text),
                    );
                }
            };
            (main $line:expr) => {
                write_str!(
                    result,
                    "- **Main Screen {}:** {}\n",
                    $line.index() + 1,
                    escape_markdown($line.text()),
                );
            };
            (chime $index:expr, $opt:expr) => {
                write_str!(
                    result,
                    "- **{:02}:00 Notification:** {}\n",
                    $index,
                    escape_markdown($opt),
                );
            };
        }

        impl_view_part_fn!(self, words, add);

        if result.is_empty() {
            result.push_str("<nothing>");
        }

        result
    }

    /// Determines whether this part shows any lines.
    fn has_texts(self, words: &SpecialSecretary) -> bool {
        macro_rules! check {
            ($_:literal, $key:ident) => {
                if words.$key.is_some() {
                    return true;
                }
            };
            ($_:ident $arg:expr $(, $arg2:expr)?) => {
                // ignore arg, we only care that the list is non-empty
                _ = ($arg $(, $arg2)?);
                return true;
            };
        }

        impl_view_part_fn!(self, words, check);
        false
    }
}

impl ButtonArgsReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        acknowledge_unloaded(&ctx).await?;

        let azur = ctx.data.config().azur()?;
        let ship = azur
            .game_data()
            .special_secretary_by_id(self.secretary_id)
            .ok_or(AzurParseError::SpecialSecretary)?;

        let create = self.create_with_sectary(ctx.data, ship);
        ctx.edit(create.into()).await
    }
}
