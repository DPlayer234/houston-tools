use azur_lane::ship::*;
use azur_lane::Faction;

use crate::buttons::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View {
    page: u16,
    filter: Filter
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Filter {
    pub name: Option<String>,
    pub faction: Option<Faction>,
    pub hull_type: Option<HullType>,
    pub rarity: Option<ShipRarity>,
    pub has_augment: Option<bool>
}

const PAGE_SIZE: usize = 15;

impl View {
    pub fn new(filter: Filter) -> View {
        View { page: 0, filter }
    }

    pub fn modify_with_iter<'a>(mut self, create: CreateReply, iter: impl Iterator<Item = &'a ShipData>) -> CreateReply {
        let mut desc = String::new();
        let mut options = Vec::new();
        let mut has_next = false;

        for ship in iter {
            if options.len() >= PAGE_SIZE {
                has_next = true;
                break
            }

            desc.push_str("- ");
            desc.push_str(&ship.name);
            desc.push('\n');

            let view_ship = AsNewMessage::new(&super::ship::View::new(ship.group_id));
            options.push(CreateSelectMenuOption::new(&ship.name, view_ship.to_custom_id()));
        }

        let embed = CreateEmbed::new()
            .title("Ships")
            .footer(CreateEmbedFooter::new(format!("Page {}", self.page + 1)))
            .description(desc)
            .color(DEFAULT_EMBED_COLOR);

        let options = CreateSelectMenuKind::String { options };
        let mut rows = vec![
            CreateActionRow::SelectMenu(CreateSelectMenu::new(self.to_custom_id(), options).placeholder("View ship..."))
        ];

        if self.page > 0 || has_next {
            rows.insert(0, CreateActionRow::Buttons(vec![
                if self.page > 0 {
                    self.new_button(utils::field_mut!(Self: page), self.page - 1, |_| 1)
                } else {
                    CreateButton::new("#no-back").disabled(true)
                }.emoji('◀'),

                if has_next {
                    self.new_button(utils::field_mut!(Self: page), self.page + 1, |_| 2)
                } else {
                    CreateButton::new("#no-forward").disabled(true)
                }.emoji('▶')
            ]));
        }

        create.embed(embed).components(rows)
    }
}

impl ButtonArgsModify for View {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        let filtered = self.filter
            .iterate(data.azur_lane())
            .skip(PAGE_SIZE * usize::from(self.page));

        Ok(self.modify_with_iter(create, filtered))
    }
}

impl Filter {
    fn iterate<'a>(&self, data: &'a HAzurLane) -> Box<dyn Iterator<Item = &'a ShipData> + 'a> {
        let predicate = self.predicate(data);
        match self.name {
            Some(ref name) => Box::new(data.ships_by_prefix(name.as_str()).filter(predicate)),
            None => Box::new(data.ship_list.iter().filter(predicate))
        }
    }

    fn predicate<'a>(&self, data: &'a HAzurLane) -> Box<dyn FnMut(&&ShipData) -> bool + 'a> {
        macro_rules! def_and_filter {
            ($fn_name:ident: $field:ident => $next:ident) => {
                fn $fn_name<'a>(f: &Filter, data: &'a HAzurLane, mut base: impl FnMut(&&ShipData) -> bool + 'a) -> Box<dyn FnMut(&&ShipData) -> bool + 'a> {
                    match f.$field {
                        Some(filter) => $next(f, data, move |s| base(s) && s.$field == filter),
                        None => $next(f, data, base)
                    }
                }
            }
        }

        def_and_filter!(next_faction: faction => next_hull_type);
        def_and_filter!(next_hull_type: hull_type => next_rarity);
        def_and_filter!(next_rarity: rarity => finish);

        fn finish<'a>(f: &Filter, data: &'a HAzurLane, mut base: impl FnMut(&&ShipData) -> bool + 'a) -> Box<dyn FnMut(&&ShipData) -> bool + 'a> {
            match f.has_augment {
                Some(filter) => {
                    Box::new(move |s| base(s) && data.augment_by_ship_id(s.group_id).is_some() == filter)
                }
                None => Box::new(base)
            }
        }

        next_faction(self, data, |_| true)
    }
}
