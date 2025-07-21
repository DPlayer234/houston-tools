use azur_lane::secretary::*;
use utils::text::WriteStr as _;

use super::AzurParseError;
use crate::buttons::prelude::*;
use crate::config::emoji;

/// Views ship lines.
#[derive(Debug, Clone, Serialize, Deserialize, ConstBuilder)]
pub struct View<'v> {
    pub secretary_id: u32,
    #[builder(default = ViewPart::Main1)]
    pub part: ViewPart,
    #[serde(borrow)]
    #[builder(default = None, setter(strip_option))]
    back: Option<Nav<'v>>,
}

/// Which part of the lines to display.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ViewPart {
    Main1,
    Main2,
    Holidays,
    Chime1,
    Chime2,
}

impl View<'_> {
    pub fn create_with_sectary<'a>(
        mut self,
        data: &'a HBotData,
        secretary: &'a SpecialSecretary,
    ) -> CreateReply<'a> {
        let mut components = CreateComponents::new();

        components.push(self.get_main_field(secretary));

        components.push(CreateActionRow::buttons(vec![
            self.button_with_part(ViewPart::Main1, secretary, "1", "< 1 >"),
            self.button_with_part(ViewPart::Main2, secretary, "2", "< 2 >"),
            self.button_with_part(ViewPart::Holidays, secretary, "3", "< 3 >"),
            self.button_with_part(ViewPart::Chime1, secretary, "4", "< 4 >"),
            self.button_with_part(ViewPart::Chime2, secretary, "5", "< 5 >"),
        ]));

        if let Some(back) = &self.back {
            let button = CreateButton::new(back.to_custom_id())
                .emoji(emoji::back())
                .label("Back");

            components.push(CreateSeparator::new(true));
            components.push(CreateActionRow::buttons(vec![button]));
        }

        CreateReply::new().components_v2(components![
            CreateContainer::new(components).accent_color(data.config().embed_color)
        ])
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

    fn get_main_field<'a>(&self, secretary: &SpecialSecretary) -> CreateTextDisplay<'a> {
        let mut content = format!("### {}\n", secretary.name);
        self.part.append_description(&mut content, secretary);

        CreateTextDisplay::new(content)
    }
}

/// Higher-order macro to share code logic for [`ViewPart`] functions.
macro_rules! impl_view_part_fn {
    ($self:expr, $words:expr, $add:ident) => {
        match $self {
            ViewPart::Main1 => {
                $add!("Login", &$words.login);

                for line in &$words.main_screen {
                    $add!(main line);
                }

                $add!("Touch", &$words.touch);
            }
            ViewPart::Main2 => {
                $add!("Mission Reminder", &$words.mission_reminder);
                $add!("Mission Complete", &$words.mission_complete);
                $add!("Mail Reminder", &$words.mail_reminder);
                $add!("Return to Port", &$words.return_to_port);
                $add!("Commission Complete", &$words.commission_complete);
            }
            ViewPart::Holidays => {
                $add!("Christmas", &$words.christmas);
                $add!("New Year's Eve", &$words.new_years_eve);
                $add!("New Year's Day", &$words.new_years_day);
                $add!("Valentine's Day", &$words.valentines);
                $add!("Mid-Autumn Festival", &$words.mid_autumn_festival);
                $add!("Halloween", &$words.halloween);
                $add!("Event Reminder", &$words.event_reminder);
                $add!("Change Module", &$words.change_module);
            }
            ViewPart::Chime1 => {
                if let Some(chime) = &$words.chime {
                    for (index, opt) in (0..12u8).zip(chime.iter()) {
                        $add!(chime index, opt);
                    }
                }
            }
            ViewPart::Chime2 => {
                if let Some(chime) = &$words.chime {
                    for (index, opt) in (0..24u8).zip(chime.iter()).skip(12) {
                        $add!(chime index, opt);
                    }
                }
            }
        }
    };
}

impl ViewPart {
    /// Creates the embed description for the current state.
    fn append_description(self, result: &mut String, words: &SpecialSecretary) {
        use crate::fmt::discord::escape_markdown;

        let len = result.len();

        // avoid duplicating the entire basic text code a million times
        fn basic(result: &mut String, label: &str, text: &str) {
            let text = escape_markdown(text);
            writeln!(result, "- **{label}:** {text}");
        }

        fn chime(result: &mut String, hour: u8, text: &str) {
            let text = escape_markdown(text);
            writeln!(result, "- **{hour:02}:00 Notification:** {text}");
        }

        macro_rules! add {
            ($label:literal, $text:expr) => {
                if let Some(text) = $text {
                    basic(result, $label, text);
                }
            };
            (main $line:expr) => {
                let index = $line.index() + 1;
                let text = escape_markdown($line.text());
                writeln!(result, "- **Main Screen {index}:** {text}");
            };
            (chime $index:expr, $opt:expr) => {
                chime(result, $index, $opt);
            };
        }

        impl_view_part_fn!(self, words, add);

        if len == result.len() {
            result.push_str("<nothing>");
        }
    }

    /// Determines whether this part shows any lines.
    fn has_texts(self, words: &SpecialSecretary) -> bool {
        macro_rules! check {
            ($_:literal, $text:expr) => {
                if $text.is_some() {
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

button_value!(for<'v> View<'v>, 21);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        let azur = ctx.data.config().azur()?;
        let ship = azur
            .game_data()
            .special_secretary_by_id(self.secretary_id)
            .ok_or(AzurParseError::SpecialSecretary)?;

        let create = self.create_with_sectary(ctx.data, ship);
        ctx.edit(create.into()).await
    }
}
