use std::fmt;

use azur_lane::GameServer;
use azur_lane::ship::*;
use utils::text::WriteStr as _;

use super::AzurParseError;
use crate::buttons::prelude::*;
use crate::config::emoji;
use crate::fmt::Join;
use crate::modules::azur::{GameData, LoadedConfig};

/// Views ship lines.
#[derive(Debug, Clone, Serialize, Deserialize, ConstBuilder)]
pub struct View<'v> {
    ship_id: u32,
    #[builder(default = 0)]
    skin_index: u8,
    #[builder(default = ViewPart::Info)]
    part: ViewPart,
    #[builder(default = false)]
    extra: bool,
    #[serde(borrow)]
    back: Nav<'v>,
    #[serde(default)]
    #[builder(default = GameServer::Unknown)]
    server: GameServer,
}

/// Which part of the lines to display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewPart {
    Info,
    Main1,
    Main2,
    Affinity,
    Combat,
}

impl View<'_> {
    fn edit_with_ship<'a>(
        self,
        ctx: ButtonContext<'a>,
        azur: LoadedConfig<'a>,
        ship: &'a ShipData,
        skin: &'a ShipSkin,
    ) -> Result<EditReply<'a>> {
        let mut create = EditReply::new();

        let thumbnail_key =
            if let Some(image_data) = azur.game_data().get_chibi_image(&skin.image_key) {
                if Some(skin.image_key.as_str()) != super::get_ship_preview_name(ctx) {
                    let filename = format!("{}.webp", skin.image_key);
                    create = create.new_attachment(CreateAttachment::bytes(image_data, filename));
                }
                Some(skin.image_key.as_str())
            } else {
                create = create.clear_attachments();
                None
            };

        Ok(create.components_v2(self.with_ship(azur, ship, skin, thumbnail_key)?))
    }

    fn with_ship<'a>(
        mut self,
        azur: LoadedConfig<'a>,
        ship: &'a ShipData,
        skin: &'a ShipSkin,
        thumbnail_key: Option<&str>,
    ) -> Result<CreateComponents<'a>> {
        let (words, words_extra) = skin.words(self.server).context("skin has no words set")?;

        self.server = words.server;
        let words = match words_extra {
            Some(words) if self.extra => words,
            _ => {
                self.extra = false;
                words
            },
        };

        self.try_redirect_to_non_empty_part(words);

        let mut components = CreateComponents::new();

        components.push(self.get_main_field(azur, skin, words, thumbnail_key));

        components.push(CreateActionRow::buttons(vec![
            self.button_with_part(ViewPart::Info, words, "1", "< 1 >"),
            self.button_with_part(ViewPart::Main1, words, "2", "< 2 >"),
            self.button_with_part(ViewPart::Main2, words, "3", "< 3 >"),
            self.button_with_part(ViewPart::Affinity, words, "4", "< 4 >"),
            self.button_with_part(ViewPart::Combat, words, "5", "< 5 >"),
        ]));

        if skin.words.len() > 1 {
            components.push(CreateActionRow::buttons(
                skin.words
                    .iter()
                    .take(5)
                    .map(|w| self.button_with_server(w.server))
                    .collect::<Vec<_>>(),
            ));
        }

        components.push(CreateSeparator::new(true));

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

        let nav_row = CreateButton::new(self.back.to_custom_id())
            .emoji(emoji::back())
            .label("Back");

        let mut nav_row = vec![nav_row];

        if words_extra.is_some() {
            nav_row.push(self.button_with_extra(false).label("Base"));
            nav_row.push(self.button_with_extra(true).label("EX"));
        }

        components.push(CreateActionRow::buttons(nav_row));

        Ok(components![
            CreateContainer::new(components).accent_color(ship.rarity.color_rgb())
        ])
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

    /// Creates a button that redirects to lines for a different game server.
    fn button_with_server<'a>(&mut self, server: GameServer) -> CreateButton<'a> {
        self.new_button(|s| &mut s.server, server, |u| u as u16)
            .style(ButtonStyle::Secondary)
            .label(server.label())
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
        if !self.part.has_texts(words)
            && let Some(part) = first_non_empty_part(words)
        {
            self.part = part;
        }
    }

    fn get_main_field<'a>(
        &self,
        azur: LoadedConfig<'a>,
        skin: &'a ShipSkin,
        words: &ShipSkinWords,
        thumbnail_key: Option<&str>,
    ) -> CreateComponent<'a> {
        let label = if self.extra { "EX Lines" } else { "Lines" };

        let mut content = format!("### {} [{label}]\n", skin.name);
        self.part
            .append_description(&mut content, azur.game_data(), words);

        let content = CreateTextDisplay::new(content);

        if let Some(thumbnail_key) = thumbnail_key {
            let url = format!("attachment://{thumbnail_key}.webp");
            let media = CreateUnfurledMediaItem::new(url);
            let thumbnail = CreateThumbnail::new(media);

            CreateSection::new(
                section_components![content],
                CreateSectionAccessory::Thumbnail(thumbnail),
            )
            .into_component()
        } else {
            content.into_component()
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
                $add!("Liked Gift", &$words.gift_prefer);
                $add!("Disliked Gift", &$words.gift_dislike);
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
    fn append_description(self, result: &mut String, game_data: &GameData, words: &ShipSkinWords) {
        use crate::fmt::discord::escape_markdown;

        let len = result.len();

        // avoid duplicating the entire basic text code a million times
        fn basic(result: &mut String, label: &str, text: &str) {
            let text = escape_markdown(text);
            writeln!(result, "- **{label}:** {text}");
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
            (couple $opt:expr) => {
                let label = ship_couple_encourage_label(game_data, $opt);
                let text = escape_markdown(&$opt.line);
                writeln!(result, "- **{label}:** {text}");
            };
        }

        impl_view_part_fn!(self, words, add);

        if len == result.len() {
            result.push_str("<nothing>");
        }
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

button_value!(for<'v> View<'v>, 4);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        let data = ctx.data_ref();
        let azur = data.config().azur()?;

        let ship = azur
            .game_data()
            .ship_by_id(self.ship_id)
            .ok_or(AzurParseError::Ship)?;

        let skin = ship
            .skins
            .get(usize::from(self.skin_index))
            .ok_or(AzurParseError::Ship)?;

        let edit = self.edit_with_ship(ctx, azur, ship, skin)?;
        ctx.edit(edit).await?;
        Ok(())
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
        _ => {
            write!(f, "Unknown couple encourage")
        },
    })
}
