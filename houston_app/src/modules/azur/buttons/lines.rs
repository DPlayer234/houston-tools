use azur_lane::ship::*;
use utils::text::write_str::*;

use super::{AzurParseError, acknowledge_unloaded};
use crate::buttons::prelude::*;
use crate::config::emoji;
use crate::fmt::Join;
use crate::helper::discord::create_string_select_menu_row;
use crate::modules::azur::{GameData, LoadedConfig};

/// Views ship lines.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub ship_id: u32,
    pub skin_index: u8,
    pub part: ViewPart,
    pub extra: bool,
    pub back: CustomData,
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

impl View {
    /// Creates a new instance including a button to go back with some custom
    /// ID.
    pub fn with_back(ship_id: u32, back: CustomData) -> Self {
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
            Some(words) if self.extra => {
                self.try_redirect_to_non_empty_part(words);
                words
            },
            _ => {
                self.extra = false;
                &skin.words
            },
        };

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
            let options: Vec<_> = ship
                .skins
                .iter()
                .take(25)
                .enumerate()
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
        index: usize,
    ) -> CreateSelectMenuOption<'a> {
        // Just as-cast the index to u8 since we'd have problems long before an
        // overflow.
        #[expect(clippy::cast_possible_truncation)]
        self.new_select_option(&skin.name, |s| &mut s.skin_index, index as u8)
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
                $add!("Description", description);
                $add!("Profile", introduction);
                $add!("Acquisition", acquisition);
            }
            ViewPart::Main1 => {
                $add!("Login", login);

                for line in &$words.main_screen {
                    $add!(main line);
                }

                $add!("Touch", touch);
                $add!("Special Touch", special_touch);
                $add!("Rub", rub);
            }
            ViewPart::Main2 => {
                $add!("Mission Reminder", mission_reminder);
                $add!("Mission Complete", mission_complete);
                $add!("Mail Reminder", mail_reminder);
                $add!("Return to Port", return_to_port);
                $add!("Commission Complete", commission_complete);
            }
            ViewPart::Affinity => {
                $add!("Details", details);
                $add!("Disappointed", disappointed);
                $add!("Stranger", stranger);
                $add!("Friendly", friendly);
                $add!("Crush", crush);
                $add!("Love", love);
                $add!("Oath", oath);
            }
            ViewPart::Combat => {
                $add!("Enhance", enhance);
                $add!("Flagship Fight", flagship_fight);
                $add!("Victory", victory);
                $add!("Defeat", defeat);
                $add!("Skill", skill);
                $add!("Low Health", low_health);

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
            (couple $opt:expr) => {
                write_str!(
                    result,
                    "- **{}:** {}\n",
                    get_label_for_ship_couple_encourage(game_data, $opt),
                    escape_markdown(&$opt.line),
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
    fn has_texts(self, words: &ShipSkinWords) -> bool {
        macro_rules! check {
            ($_:literal, $key:ident) => {
                if words.$key.is_some() {
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

impl ButtonArgsReply for View {
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
fn get_label_for_ship_couple_encourage(game_data: &GameData, opt: &ShipCoupleEncourage) -> String {
    fn fmt_sortie_count<T>(
        label: &str,
        amount: u32,
        items: &[T],
        to_name: impl Fn(&T) -> &str,
    ) -> String {
        let plural = if amount != 1 { "s" } else { "" };
        let fmt = Join::OR.display_as(items, to_name);
        format!("Sortie with {amount} more {fmt}{label}{plural}")
    }

    match &opt.condition {
        ShipCouple::ShipGroup(ship_ids) => {
            let get_name = |&id| {
                game_data
                    .ship_by_id(id)
                    .map_or("<unknown>", |s| s.name.as_str())
            };

            let amount = opt.amount;
            if ship_ids.len() == amount {
                let fmt = Join::AND.display_as(ship_ids, get_name);
                format!("Sortie with {fmt}")
            } else {
                let fmt = Join::OR.display_as(ship_ids, get_name);
                format!("Sortie with {amount} of {fmt}")
            }
        },
        ShipCouple::HullType(hull_types) => {
            fmt_sortie_count("", opt.amount, hull_types, |h| h.designation())
        },
        ShipCouple::Rarity(rarities) => {
            fmt_sortie_count(" ship", opt.amount, rarities, |r| r.name())
        },
        ShipCouple::Faction(factions) => {
            fmt_sortie_count(" ship", opt.amount, factions, |f| f.name())
        },
        ShipCouple::Illustrator => {
            format!(
                "Sortie with {} more ships by the same illustrator",
                opt.amount,
            )
        },
    }
}
