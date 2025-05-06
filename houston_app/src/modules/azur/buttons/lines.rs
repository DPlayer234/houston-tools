use std::fmt;

use azur_lane::ship::*;
use utils::text::WriteStr as _;

use super::{AzurParseError, acknowledge_unloaded};
use crate::buttons::prelude::*;
use crate::config::emoji;
use crate::fmt::Join;
use crate::helper::discord::create_string_select_menu_row;
use crate::modules::azur::{GameData, LoadedConfig};

/// Views ship lines.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View<'v> {
    pub ship_id: u32,
    pub skin_index: u8,
    pub part: ViewPart,
    pub extra: bool,
    #[serde(borrow)]
    pub back: Nav<'v>,
}

/// Which part of the lines to display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ViewPart {
    Info,
    Main1,
    Main2,
    Affinity,
    Combat,
}

impl<'v> View<'v> {
    /// Creates a new instance including a button to go back with some custom
    /// ID.
    pub fn with_back(ship_id: u32, back: Nav<'v>) -> Self {
        Self {
            ship_id,
            skin_index: 0,
            part: ViewPart::Info,
            extra: false,
            back,
        }
    }

    fn edit_with_ship<'a>(
        self,
        ctx: &ButtonContext<'a>,
        azur: LoadedConfig<'a>,
        ship: &'a ShipData,
        skin: &'a ShipSkin,
    ) -> EditReply<'a> {
        let (mut embed, components) = self.with_ship(azur, ship, skin);
        let mut create = EditReply::new();

        if let Some(image_data) = azur.game_data().get_chibi_image(&skin.image_key) {
            embed = embed.thumbnail(format!("attachment://{}.webp", skin.image_key));

            if Some(skin.image_key.as_str()) != super::get_ship_preview_name(ctx) {
                let filename = format!("{}.webp", skin.image_key);
                create = create.new_attachment(CreateAttachment::bytes(image_data, filename));
            }
        } else {
            create = create.clear_attachments();
        }

        create.embed(embed).components(components)
    }

    fn with_ship<'a>(
        mut self,
        azur: LoadedConfig<'a>,
        ship: &'a ShipData,
        skin: &'a ShipSkin,
    ) -> (CreateEmbed<'a>, Vec<CreateActionRow<'a>>) {
        let words = match &skin.words_extra {
            Some(words) if self.extra => words,
            _ => {
                self.extra = false;
                &skin.words
            },
        };

        self.try_redirect_to_non_empty_part(words);
        let embed = CreateEmbed::new()
            .color(ship.rarity.color_rgb())
            .author(azur.wiki_urls().ship(ship))
            .description(self.part.get_description(azur.game_data(), words));

        let mut components = Vec::new();

        let top_row = CreateButton::new(self.back.to_custom_id())
            .emoji(emoji::back())
            .label("Back");
        let mut top_row = vec![top_row];

        if skin.words_extra.is_some() {
            top_row.push(self.button_with_extra(false).label("Base"));
            top_row.push(self.button_with_extra(true).label("EX"));
        }

        if !top_row.is_empty() {
            components.push(CreateActionRow::buttons(top_row));
        }

        components.push(CreateActionRow::buttons(vec![
            self.button_with_part(ViewPart::Info, words, "1", "< 1 >"),
            self.button_with_part(ViewPart::Main1, words, "2", "< 2 >"),
            self.button_with_part(ViewPart::Main2, words, "3", "< 3 >"),
            self.button_with_part(ViewPart::Affinity, words, "4", "< 4 >"),
            self.button_with_part(ViewPart::Combat, words, "5", "< 5 >"),
        ]));

        if ship.skins.len() > 1 {
            let options: Vec<_> = (0..25u8)
                .zip(&ship.skins)
                .map(|(index, skin)| self.select_with_skin_index(skin, index))
                .collect();

            components.push(create_string_select_menu_row(
                self.to_custom_id(),
                options,
                &skin.name,
            ));
        }

        (embed, components)
    }

    /// Creates a button that redirects to a different Base/EX state.
    fn button_with_extra<'a>(&mut self, extra: bool) -> CreateButton<'a> {
        self.new_button(|s| &mut s.extra, extra, bool::into)
    }

    /// Creates a button that redirects to a different viewed part.
    fn button_with_part<'a>(
        &mut self,
        part: ViewPart,
        words: &ShipSkinWords,
        label: &'a str,
        active_label: &'a str,
    ) -> CreateButton<'a> {
        let button = self
            .new_button(|s| &mut s.part, part, |u| u as u16)
            .style(ButtonStyle::Secondary)
            .label(label);
        if !part.has_texts(words) {
            button.disabled(true)
        } else if self.part == part {
            button.label(active_label)
        } else {
            button
        }
    }

    /// Creates a button that redirects to a different skin's lines.
    fn select_with_skin_index<'a>(
        &mut self,
        skin: &'a ShipSkin,
        index: u8,
    ) -> CreateSelectMenuOption<'a> {
        self.new_select_option(&skin.name, |s| &mut s.skin_index, index)
    }

    /// Attempts to change to a part that has texts, if the view isn't already
    /// on one.
    fn try_redirect_to_non_empty_part(&mut self, words: &ShipSkinWords) {
        if !self.part.has_texts(words) {
            if let Some(part) = first_non_empty_part(words) {
                self.part = part;
            }
        }
    }
}

fn first_non_empty_part(words: &ShipSkinWords) -> Option<ViewPart> {
    macro_rules! check {
        ($part:expr) => {
            if $part.has_texts(words) {
                return Some($part);
            }
        };
    }

    check!(ViewPart::Info);
    check!(ViewPart::Main1);
    check!(ViewPart::Main2);
    check!(ViewPart::Affinity);
    check!(ViewPart::Combat);
    None
}

/// Higher-order macro to share code logic for [`ViewPart`] functions.
macro_rules! impl_view_part_fn {
    ($self:expr, $words:expr, $add:ident) => {
        match $self {
            ViewPart::Info => {
                $add!("Description", &$words.description);
                $add!("Profile", &$words.introduction);
                $add!("Acquisition", &$words.acquisition);
            }
            ViewPart::Main1 => {
                $add!("Login", &$words.login);

                for line in &$words.main_screen {
                    $add!(main line);
                }

                $add!("Touch", &$words.touch);
                $add!("Special Touch", &$words.special_touch);
                $add!("Rub", &$words.rub);
            }
            ViewPart::Main2 => {
                $add!("Mission Reminder", &$words.mission_reminder);
                $add!("Mission Complete", &$words.mission_complete);
                $add!("Mail Reminder", &$words.mail_reminder);
                $add!("Return to Port", &$words.return_to_port);
                $add!("Commission Complete", &$words.commission_complete);
            }
            ViewPart::Affinity => {
                $add!("Details", &$words.details);
                $add!("Disappointed", &$words.disappointed);
                $add!("Stranger", &$words.stranger);
                $add!("Friendly", &$words.friendly);
                $add!("Crush", &$words.crush);
                $add!("Love", &$words.love);
                $add!("Oath", &$words.oath);
            }
            ViewPart::Combat => {
                $add!("Enhance", &$words.enhance);
                $add!("Flagship Fight", &$words.flagship_fight);
                $add!("Victory", &$words.victory);
                $add!("Defeat", &$words.defeat);
                $add!("Skill", &$words.skill);
                $add!("Low Health", &$words.low_health);

                for opt in &$words.couple_encourage {
                    $add!(couple opt);
                }
            }
        }
    };
}

impl ViewPart {
    /// Creates the embed description for the current state.
    fn get_description(self, game_data: &GameData, words: &ShipSkinWords) -> String {
        use crate::fmt::discord::escape_markdown;

        let mut result = String::new();

        // avoid duplicating the entire basic text code a million times
        fn basic(result: &mut String, label: &str, text: &str) {
            let text = escape_markdown(text);
            writeln!(result, "- **{label}:** {text}");
        }

        macro_rules! add {
            ($label:literal, $text:expr) => {
                if let Some(text) = $text {
                    basic(&mut result, $label, text);
                }
            };
            (main $line:expr) => {
                let index = $line.index() + 1;
                let text = escape_markdown($line.text());
                writeln!(result, "- **Main Screen {index}:** {text}");
            };
            (couple $opt:expr) => {
                let label = ship_couple_encourage_label(game_data, $opt);
                let text = escape_markdown(&$opt.line);
                writeln!(result, "- **{label}:** {text}");
            };
        }

        impl_view_part_fn!(self, words, add);

        if result.is_empty() {
            result.push_str("<nothing>");
        }

        result
    }

    /// Determines whether this part shows any lines.
    fn has_texts(self, words: &ShipSkinWords) -> bool {
        macro_rules! check {
            ($_:literal, $text:expr) => {
                if $text.is_some() {
                    return true;
                }
            };
            ($_:ident $arg:expr) => {
                // ignore arg, we only care that the list is non-empty
                _ = $arg;
                return true;
            };
        }

        impl_view_part_fn!(self, words, check);
        false
    }
}

button_value!(View<'_>, 4);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        acknowledge_unloaded(&ctx).await?;

        let azur = ctx.data.config().azur()?;

        let ship = azur
            .game_data()
            .ship_by_id(self.ship_id)
            .ok_or(AzurParseError::Ship)?;

        let skin = ship
            .skins
            .get(usize::from(self.skin_index))
            .ok_or(AzurParseError::Ship)?;

        let edit = self.edit_with_ship(&ctx, azur, ship, skin);
        ctx.edit(edit).await
    }
}

/// Creates a label for a couple line.
fn ship_couple_encourage_label(
    game_data: &GameData,
    opt: &ShipCoupleEncourage,
) -> impl fmt::Display {
    fn fmt_sortie_count<T>(
        f: &mut fmt::Formatter<'_>,
        label: &str,
        amount: u32,
        items: &[T],
        to_name: impl Fn(&T) -> &str,
    ) -> fmt::Result {
        let plural = if amount != 1 { "s" } else { "" };
        let fmt = Join::OR.display_as(items, to_name);
        write!(f, "Sortie with {amount} more {fmt}{label}{plural}")
    }

    let condition = &opt.condition;
    let amount = opt.amount;

    utils::text::from_fn(move |f| match condition {
        ShipCouple::ShipGroup(ship_ids) => {
            let get_name = |&id| {
                game_data
                    .ship_by_id(id)
                    .map_or("<unknown>", |s| s.name.as_str())
            };

            if ship_ids.len() == amount {
                let fmt = Join::AND.display_as(ship_ids, get_name);
                write!(f, "Sortie with {fmt}")
            } else {
                let fmt = Join::OR.display_as(ship_ids, get_name);
                write!(f, "Sortie with {amount} of {fmt}")
            }
        },
        ShipCouple::HullType(hull_types) => {
            fmt_sortie_count(f, "", amount, hull_types, |h| h.designation())
        },
        ShipCouple::Rarity(rarities) => {
            fmt_sortie_count(f, " ship", amount, rarities, |r| r.name())
        },
        ShipCouple::Faction(factions) => {
            fmt_sortie_count(f, " ship", amount, factions, |f| f.name())
        },
        ShipCouple::Illustrator => {
            write!(f, "Sortie with {amount} more ships by the same illustrator")
        },
    })
}
