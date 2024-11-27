use azur_lane::ship::*;
use utils::text::write_str::*;

use super::get_ship_preview_name;
use super::AzurParseError;
use crate::buttons::prelude::*;
use crate::fmt::JoinNatural;

/// Views ship lines.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub ship_id: u32,
    pub skin_index: u8,
    pub part: ViewPart,
    pub extra: bool,
    pub back: Option<CustomData>
}

/// Which part of the lines to display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ViewPart {
    Info,
    Main1,
    Main2,
    Affinity,
    Combat
}

impl View {
    /// Creates a new instance.
    #[allow(dead_code)] // planned for future use
    pub fn new(ship_id: u32) -> Self {
        Self { ship_id, skin_index: 0, part: ViewPart::Info, extra: false, back: None }
    }

    /// Creates a new instance including a button to go back with some custom ID.
    pub fn with_back(ship_id: u32, back: CustomData) -> Self {
        Self { ship_id, skin_index: 0, part: ViewPart::Info, extra: false, back: Some(back) }
    }

    /// Modifies the create-reply with preresolved ship and skin data.
    pub fn create_with_ship<'a>(
        self,
        data: &'a HBotData,
        ship: &'a ShipData,
        skin: &'a ShipSkin,
    ) -> CreateReply<'a> {
        let (mut embed, components) = self.with_ship(data, ship, skin);
        let mut create = CreateReply::new();

        if let Some(image_data) = data.azur_lane().get_chibi_image(&skin.image_key) {
            create = create.attachment(CreateAttachment::bytes(image_data, format!("{}.webp", skin.image_key)));
            embed = embed.thumbnail(format!("attachment://{}.webp", skin.image_key));
        }

        create.embed(embed).components(components)
    }

    fn edit_with_ship<'a>(
        self,
        ctx: &ButtonContext<'a>,
        ship: &'a ShipData,
        skin: &'a ShipSkin,
    ) -> EditReply<'a> {
        let (mut embed, components) = self.with_ship(ctx.data, ship, skin);
        let mut create = EditReply::new();

        if let Some(image_data) = ctx.data.azur_lane().get_chibi_image(&skin.image_key) {
            embed = embed.thumbnail(format!("attachment://{}.webp", skin.image_key));

            if Some(skin.image_key.as_str()) != get_ship_preview_name(ctx) {
                create = create.new_attachment(CreateAttachment::bytes(image_data, format!("{}.webp", skin.image_key)));
            }
        } else {
            create = create.clear_attachments();
        }

        create.embed(embed).components(components)
    }

    fn with_ship<'a>(
        mut self,
        data: &'a HBotData,
        ship: &'a ShipData,
        skin: &'a ShipSkin,
    ) -> (CreateEmbed<'a>, Vec<CreateActionRow<'a>>) {
        let words = match (&self, skin) {
            (Self { extra: true, .. }, ShipSkin { words_extra: Some(words), .. } ) => words.as_ref(),
            _ => { self.extra = false; &skin.words }
        };

        let embed = CreateEmbed::new()
            .color(ship.rarity.color_rgb())
            .author(super::get_ship_wiki_url(ship))
            .description(self.part.get_description(data, words));

        let mut components = Vec::new();

        let mut top_row = Vec::new();
        if let Some(back) = &self.back {
            top_row.push(CreateButton::new(back.to_custom_id()).emoji('⏪').label("Back"));
        }

        if skin.words_extra.is_some() {
            top_row.push(self.button_with_extra(false).label("Base"));
            top_row.push(self.button_with_extra(true).label("EX"));
        }

        if !top_row.is_empty() {
            components.push(CreateActionRow::buttons(top_row));
        }

        components.push(CreateActionRow::buttons(vec![
            self.button_with_part(ViewPart::Info, words).label("1").style(ButtonStyle::Secondary),
            self.button_with_part(ViewPart::Main1, words).label("2").style(ButtonStyle::Secondary),
            self.button_with_part(ViewPart::Main2, words).label("3").style(ButtonStyle::Secondary),
            self.button_with_part(ViewPart::Affinity, words).label("4").style(ButtonStyle::Secondary),
            self.button_with_part(ViewPart::Combat, words).label("5").style(ButtonStyle::Secondary),
        ]));

        if ship.skins.len() > 1 {
            let options: Vec<_> = ship.skins.iter().take(25).enumerate()
                .map(|(index, skin)| self.select_with_skin_index(skin, index))
                .collect();

            components.push(super::create_string_select_menu_row(
                self.to_custom_id(),
                options,
                &skin.name,
            ));
        }

        (embed, components)
    }

    /// Creates a button that redirects to a different Base/EX state.
    fn button_with_extra<'a>(&mut self, extra: bool) -> CreateButton<'a> {
        self.new_button(utils::field_mut!(Self: extra), extra, bool::into)
    }

    /// Creates a button that redirects to a different viewed part.
    fn button_with_part<'a>(&mut self, part: ViewPart, words: &ShipSkinWords) -> CreateButton<'a> {
        let disabled = self.part == part || !part.has_texts(words);
        self.new_button(utils::field_mut!(Self: part), part, |u| u as u16).disabled(disabled)
    }

    /// Creates a button that redirects to a different skin's lines.
    fn select_with_skin_index<'a>(&mut self, skin: &'a ShipSkin, index: usize) -> CreateSelectMenuOption<'a> {
        // Just as-cast the index to u8 since we'd have problems long before an overflow.
        #[allow(clippy::cast_possible_truncation)]
        self.new_select_option(&skin.name, utils::field_mut!(Self: skin_index), index as u8)
    }

    fn resolve<'a>(&self, ctx: &ButtonContext<'a>) -> anyhow::Result<(&'a ShipData, &'a ShipSkin)> {
        let ship = ctx.data.azur_lane().ship_by_id(self.ship_id).ok_or(AzurParseError::Ship)?;
        let skin = ship.skins.get(usize::from(self.skin_index)).ok_or(AzurParseError::Ship)?;
        Ok((ship, skin))
    }
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
    fn get_description(self, data: &HBotData, words: &ShipSkinWords) -> String {
        use crate::fmt::discord::escape_markdown;

        let mut result = String::new();

        macro_rules! add {
            ($label:literal, $key:ident) => {
                if let Some(text) = &words.$key {
                    write_str!(result, concat!("- **", $label, ":** {}\n"), escape_markdown(text));
                }
            };
            (main $line:expr) => {
                write_str!(result, "- **Main Screen {}:** {}\n", $line.index() + 1, escape_markdown($line.text()));
            };
            (couple $opt:expr) => {
                write_str!(result, "- **{}:** {}\n", get_label_for_ship_couple_encourage(data, $opt), escape_markdown(&$opt.line));
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

impl ButtonMessage for View {
    fn create_reply(self, ctx: ButtonContext<'_>) -> anyhow::Result<CreateReply<'_>> {
        let (ship, skin) = self.resolve(&ctx)?;
        Ok(self.create_with_ship(ctx.data, ship, skin))
    }

    fn edit_reply(self, ctx: ButtonContext<'_>) -> anyhow::Result<EditReply<'_>> {
        let (ship, skin) = self.resolve(&ctx)?;
        Ok(self.edit_with_ship(&ctx, ship, skin))
    }
}

/// Creates a label for a couple line.
fn get_label_for_ship_couple_encourage(data: &HBotData, opt: &ShipCoupleEncourage) -> String {
    fn fmt_sortie_count<'a>(label: &str, amount: u32, iter: impl Iterator<Item = &'a str>) -> String {
        let plural = if amount != 1 { "s" } else { "" };
        format!(
            "Sortie with {} more {}{}{}",
            amount, JoinNatural::or(iter), label, plural,
        )
    }

    fn fmt_ships_count<'a>(amount: u32, iter: impl Iterator<Item = &'a str>) -> String {
        format!(
            "Sortie with {} of {}",
            amount, JoinNatural::or(iter),
        )
    }

    fn fmt_ships_all<'a>(iter: impl Iterator<Item = &'a str>) -> String {
        format!("Sortie with {}", JoinNatural::and(iter))
    }

    match &opt.condition {
        ShipCouple::ShipGroup(ship_ids) => {
            let ships = ship_ids.iter()
                .filter_map(|&id| data.azur_lane().ship_by_id(id))
                .map(|ship| ship.name.as_str());

            if ship_ids.len() == opt.amount.try_into().unwrap_or(0) {
                fmt_ships_all(ships)
            } else {
                fmt_ships_count(opt.amount, ships)
            }
        }
        ShipCouple::HullType(hull_types) => {
            let hull_types = hull_types.iter()
                .map(|hull_type| hull_type.designation());

            fmt_sortie_count("", opt.amount, hull_types)
        }
        ShipCouple::Rarity(rarities) => {
            let rarities = rarities.iter()
                .map(|rarity| rarity.name());

            fmt_sortie_count(" ship", opt.amount, rarities)
        }
        ShipCouple::Faction(factions) => {
            let factions = factions.iter()
                .map(|faction| faction.name());

            fmt_sortie_count(" ship", opt.amount, factions)
        }
        ShipCouple::Illustrator => {
            format!("Sortie with {} more ships by the same illustrator", opt.amount)
        }
    }
}
