use std::fmt::Write;
//use crate::internal::prelude::*;
use crate::buttons::*;
use azur_lane::ship::*;
use utils::Discard;

use super::ShipParseError;

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct ViewLines {
    pub ship_id: u32,
    pub skin_index: u32,
    pub part: ViewLinesPart,
    pub extra: bool,
    pub back: Option<String>
}

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
    pub fn new(ship_id: u32) -> Self {
        Self { ship_id, skin_index: 0, part: ViewLinesPart::Info, extra: false, back: None }
    }

    pub fn with_back(ship_id: u32, back: String) -> Self {
        Self { ship_id, skin_index: 0, part: ViewLinesPart::Info, extra: false, back: Some(back) }
    }

    pub fn modify_with_ship(mut self, data: &HBotData, create: CreateReply, ship: &ShipData, skin: &ShipSkin) -> anyhow::Result<CreateReply> {
        let words = match (&self, skin) {
            (ViewLines { extra: true, .. }, ShipSkin { words_extra: Some(words), .. } ) => words.as_ref(),
            _ => { self.extra = false; &skin.words }
        };

        let embed = CreateEmbed::new()
            .color(ship.rarity.data().color_rgb)
            .author(super::get_ship_url(ship))
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

        Ok(create.embed(embed).components(components))
    }
    
    fn button_with_extra(&self, extra: bool) -> CreateButton {
        self.new_button(utils::field!(Self: extra), extra, || Sentinel::new(0, extra as u32))
    }

    fn button_with_part(&self, part: ViewLinesPart) -> CreateButton {
        self.new_button(utils::field!(Self: part), part, || Sentinel::new(1, part as u32))
    }

    fn select_with_skin_index(&self, skin: &ShipSkin, index: usize) -> CreateSelectMenuOption {
        self.new_select_option(&skin.name, utils::field!(Self: skin_index), index as u32)
    }

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
        self.modify_with_ship(data, create, ship, skin)
    }
}

fn get_label_for_ship_couple_encourage(data: &HBotData, opt: &ShipCoupleEncourage) -> String {
    match &opt.condition {
        ShipCouple::ShipGroup(ship_ids) => {
            let ships = ship_ids.iter()
                .flat_map(|&id| data.azur_lane().ship_by_id(id))
                .map(|ship| ship.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");

            if opt.amount == 1 {
                format!("Sortie with {}", ships)
            } else {
                format!(
                    "Sortie with {} of {}",
                    opt.amount,
                    ship_ids.iter()
                        .flat_map(|&id| data.azur_lane().ship_by_id(id))
                        .map(|ship| ship.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
        ShipCouple::HullType(hull_types) => {
            let hull_types = hull_types.iter()
                .map(|hull_type| hull_type.data().designation)
                .collect::<Vec<_>>()
                .join(", ");

            format!(
                "Sortie with {} more {}",
                opt.amount,
                hull_types
            )
        }
        ShipCouple::Rarity(rarities) => {
            let rarities = rarities.iter()
                .map(|rarity| rarity.data().name)
                .collect::<Vec<_>>()
                .join(", ");

            format!(
                "Sortie with {} more {} ships",
                opt.amount,
                rarities
            )
        }
        ShipCouple::Faction(factions) => {
            let factions = factions.iter()
                .map(|faction| faction.data().name)
                .collect::<Vec<_>>()
                .join(", ");

            format!(
                "Sortie with {} more {} ships",
                opt.amount,
                factions
            )
        }
        ShipCouple::Illustrator => {
            format!("Sortie with {} more ships by the same illustrator", opt.amount)
        }
    }
}
