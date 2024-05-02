use std::fmt::Write;
//use crate::internal::prelude::*;
use crate::buttons::*;
use azur_lane::ship::*;
use utils::Discard;

use super::ShipParseError;

/// Views ship lines.
#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct ViewLines {
    pub ship_id: u32,
    pub skin_index: u32,
    pub part: ViewLinesPart,
    pub extra: bool,
    pub back: Option<String>
}

/// Which part of the lines to display.
#[derive(Debug, Clone, Copy, bitcode::Encode, bitcode::Decode, PartialEq, Eq)]
pub enum ViewLinesPart {
    Info,
    Main1,
    Main2,
    Affinity,
    Combat
}

impl From<ViewLines> for ButtonArgs {
    fn from(value: ViewLines) -> Self {
        ButtonArgs::ViewLines(value)
    }
}

impl ViewLines {
    /// Creates a new instance.
    pub fn new(ship_id: u32) -> Self {
        Self { ship_id, skin_index: 0, part: ViewLinesPart::Info, extra: false, back: None }
    }

    /// Creates a new instance including a button to go back with some custom ID.
    pub fn with_back(ship_id: u32, back: String) -> Self {
        Self { ship_id, skin_index: 0, part: ViewLinesPart::Info, extra: false, back: Some(back) }
    }

    /// Modifies the create-reply with preresolved ship and skin data.
    pub fn modify_with_ship(mut self, data: &HBotData, mut create: CreateReply, ship: &ShipData, skin: &ShipSkin) -> CreateReply {
        let words = match (&self, skin) {
            (ViewLines { extra: true, .. }, ShipSkin { words_extra: Some(words), .. } ) => words.as_ref(),
            _ => { self.extra = false; &skin.words }
        };

        let mut embed = CreateEmbed::new()
            .color(ship.rarity.color_rgb())
            .author(super::get_ship_wiki_url(ship))
            .description(self.get_description(data, words));

        let mut components = Vec::new();

        let mut top_row = Vec::new();
        if let Some(ref back) = self.back {
            top_row.push(CreateButton::new(back).emoji('⏪').label("Back"));
        }

        if skin.words_extra.is_some() {
            top_row.push(self.button_with_extra(false).label("Base"));
            top_row.push(self.button_with_extra(true).label("EX"));
        }

        if !top_row.is_empty() {
            components.push(CreateActionRow::Buttons(top_row));
        }

        components.push(CreateActionRow::Buttons(vec![
            self.button_with_part(ViewLinesPart::Info).label("1").style(ButtonStyle::Secondary),
            self.button_with_part(ViewLinesPart::Main1).label("2").style(ButtonStyle::Secondary),
            self.button_with_part(ViewLinesPart::Main2).label("3").style(ButtonStyle::Secondary),
            self.button_with_part(ViewLinesPart::Affinity).label("4").style(ButtonStyle::Secondary),
            self.button_with_part(ViewLinesPart::Combat).label("5").style(ButtonStyle::Secondary),
        ]));

        if ship.skins.len() > 1 {
            let options = CreateSelectMenuKind::String {
                options: ship.skins.iter().enumerate()
                    .map(|(index, skin)| self.select_with_skin_index(skin, index))
                    .collect()
            };

            let select = CreateSelectMenu::new(self.clone().to_custom_id(), options)
                .placeholder(&skin.name);

            components.push(CreateActionRow::SelectMenu(select));
        }

        if let Some(image_data) = data.azur_lane().get_chibi_image(&skin.image_key) {
            create = create.attachment(CreateAttachment::bytes(image_data.as_ref(), format!("{}.png", skin.image_key)));
            embed = embed.thumbnail(format!("attachment://{}.png", skin.image_key));
        }

        create.embed(embed).components(components)
    }
    
    /// Creates a button that redirects to a different Base/EX state.
    fn button_with_extra(&self, extra: bool) -> CreateButton {
        self.new_button(utils::field!(Self: extra), extra, || Sentinel::new(0, extra as u32))
    }

    /// Creates a button that redirects to a different viewed part.
    fn button_with_part(&self, part: ViewLinesPart) -> CreateButton {
        self.new_button(utils::field!(Self: part), part, || Sentinel::new(1, part as u32))
    }

    /// Creates a button that redirects to a different skin's lines.
    fn select_with_skin_index(&self, skin: &ShipSkin, index: usize) -> CreateSelectMenuOption {
        self.new_select_option(&skin.name, utils::field!(Self: skin_index), index as u32)
    }

    /// Creates the embed description for the current state.
    fn get_description(&self, data: &HBotData, words: &ShipSkinWords) -> String {
        let mut result = String::new();

        macro_rules! add {
            ($label:literal, $key:ident) => {{
                if let Some(ref text) = words.$key {
                    write!(result, concat!("- **", $label, ":** {}\n"), text).discard();
                }
            }};
            (dyn $label:literal, $($extra:tt)*) => {{
                write!(result, concat!("- **", $label, ":** {}\n"), $($extra)*).discard();
            }};
        }

        match self.part {
            ViewLinesPart::Info => {
                add!("Description", description);
                add!("Profile", introduction);
                add!("Acquisition", acquisition);
            }
            ViewLinesPart::Main1 => {
                add!("Login", login);
                
                for line in &words.main_screen {
                    add!(dyn "Main Screen {}", line.index() + 1, line.text());    
                }

                add!("Touch", touch);
                add!("Special Touch", special_touch);
                add!("Rub", rub);
            }
            ViewLinesPart::Main2 => {
                add!("Mission Reminder", mission_reminder);
                add!("Mission Complete", mission_complete);
                add!("Mail Reminder", mail_reminder);
                add!("Return to Port", return_to_port);
                add!("Commission Complete", commission_complete);
            }
            ViewLinesPart::Affinity => {
                add!("Details", details);
                add!("Disappointed", disappointed);
                add!("Stranger", stranger);
                add!("Friendly", friendly);
                add!("Crush", crush);
                add!("Love", love);
                add!("Oath", oath);
            }
            ViewLinesPart::Combat => {
                add!("Enhance", enhance);
                add!("Flagship Fight", flagship_fight);
                add!("Victory", victory);
                add!("Defeat", defeat);
                add!("Skill", skill);
                add!("Low Health", low_health);

                for opt in &words.couple_encourage {
                    let label = get_label_for_ship_couple_encourage(data, opt);
                    add!(dyn "{}", label, opt.line);
                }
            }
        }

        if result.is_empty() {
            result.push_str("<nothing>");
        }

        result
    }
}

impl ButtonArgsModify for ViewLines {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        let ship = data.azur_lane().ship_by_id(self.ship_id).ok_or(ShipParseError)?;
        let skin = ship.skins.get(self.skin_index as usize).ok_or(ShipParseError)?;
        Ok(self.modify_with_ship(data, create, ship, skin))
    }
}

/// Creates a label for a couple line.
fn get_label_for_ship_couple_encourage(data: &HBotData, opt: &ShipCoupleEncourage) -> String {
    match &opt.condition {
        ShipCouple::ShipGroup(ship_ids) => {
            let ships = ship_ids.iter()
                .flat_map(|&id| data.azur_lane().ship_by_id(id))
                .map(|ship| ship.name.as_str());

            if opt.amount as usize == ship_ids.len() {
                format!("Sortie with {}", join_natural_and(ships))
            } else {
                format!(
                    "Sortie with {} of {}",
                    opt.amount,
                    join_natural_or(ships)
                )
            }
        }
        ShipCouple::HullType(hull_types) => {
            let hull_types = hull_types.iter()
                .map(|hull_type| hull_type.designation());

            let label = if opt.amount != 1 { "s" } else { "" };
            format!(
                "Sortie with {} more {}{}",
                opt.amount,
                join_natural_or(hull_types),
                label
            )
        }
        ShipCouple::Rarity(rarities) => {
            let rarities = rarities.iter()
                .map(|rarity| rarity.name());

            let label = if opt.amount != 1 { "s" } else { "" };
            format!(
                "Sortie with {} more {} ship{}",
                opt.amount,
                join_natural_or(rarities),
                label
            )
        }
        ShipCouple::Faction(factions) => {
            let factions = factions.iter()
                .map(|faction| faction.name());

            let label = if opt.amount != 1 { "s" } else { "" };
            format!(
                "Sortie with {} more {} ship{}",
                opt.amount,
                join_natural_or(factions),
                label
            )
        }
        ShipCouple::Illustrator => {
            format!("Sortie with {} more ships by the same illustrator", opt.amount)
        }
    }
}

fn join_natural_and<'a>(iter: impl Iterator<Item = &'a str>) -> String {
    join_natural(iter, ", ", ", and ", " and ")
}

fn join_natural_or<'a>(iter: impl Iterator<Item = &'a str>) -> String {
    join_natural(iter, ", ", ", or ", " or ")
}

fn join_natural<'a>(iter: impl Iterator<Item = &'a str>, join: &str, join_last: &str, join_once: &str) -> String {
    let data = iter.collect::<Vec<_>>();
    match data.split_last() {
        None => String::new(),
        Some((&last, head)) => {
            if head.is_empty() { return last.to_owned(); }
            if head.len() == 1 { return head[0].to_owned() + join_once + last; }

            let mut result = head.join(join);
            result.push_str(join_last);
            result.push_str(last);
            result
        }
    }
}
