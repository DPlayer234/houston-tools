use azur_lane::ship::*;
use azur_lane::Faction;

use crate::buttons::*;

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct ViewFilter {
    page: u16,
    filter: ViewFilterInfo
}

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct ViewFilterInfo {
    pub faction: Option<Faction>,
    pub hull_type: Option<HullType>,
    pub rarity: Option<ShipRarity>,
    pub has_augment: Option<bool>
}

impl From<ViewFilter> for ButtonArgs {
    fn from(value: ViewFilter) -> Self {
        ButtonArgs::ViewFilter(value)
    }
}

const PAGE_SIZE: usize = 15;

impl ViewFilter {
    pub fn new(filter: ViewFilterInfo) -> ViewFilter {
        ViewFilter { page: 0, filter }
    }

    pub fn modify_with_iter<'a>(self, create: CreateReply, iter: impl Iterator<Item = &'a ShipData>) -> CreateReply {
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

            let view_ship = super::ship::ViewShip::new(ship.group_id);
            options.push(CreateSelectMenuOption::new(&ship.name, view_ship.to_custom_id()));
        }

        let embed = CreateEmbed::new()
            .title("Ships")
            .footer(CreateEmbedFooter::new(format!("Page {}", self.page + 1)))
            .description(desc)
            .color(DEFAULT_EMBED_COLOR);

        let options = CreateSelectMenuKind::String { options };
        let mut rows = vec![
            CreateActionRow::SelectMenu(CreateSelectMenu::new(self.clone().to_custom_id(), options).placeholder("View ship..."))
        ];

        if self.page > 0 || has_next {
            rows.insert(0, CreateActionRow::Buttons(vec![
                if self.page > 0 {
                    self.new_button(utils::field!(Self: page), self.page - 1, || Sentinel::new(0, 1))
                } else {
                    CreateButton::new("#no-back").disabled(true)
                }.emoji('◀'),

                if has_next {
                    self.new_button(utils::field!(Self: page), self.page + 1, || Sentinel::new(0, 2))
                } else {
                    CreateButton::new("#no-forward").disabled(true)
                }.emoji('▶')
            ]));
        }

        create.embed(embed).components(rows)
    }
}

impl ButtonArgsModify for ViewFilter {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        let mut predicate = self.filter.predicate(data);
        let filtered = data.azur_lane().ship_list.iter()
            .filter(move |&s| predicate(s))
            .skip(PAGE_SIZE * usize::from(self.page));

        Ok(self.modify_with_iter(create, filtered))
    }
}

macro_rules! def_and_filter {
    ($fn_name:ident: $field:ident => $next:ident) => {
        fn $fn_name<'a>(f: &ViewFilterInfo, data: &'a HBotData, mut base: impl FnMut(&ShipData) -> bool + 'a) -> Box<dyn FnMut(&ShipData) -> bool + 'a> {
            match f.$field {
                Some(filter) => $next(f, data, move |s| base(s) && s.$field == filter),
                None => $next(f, data, base)
            }
        }
    }
}

impl ViewFilterInfo {
    fn predicate<'a>(&self, data: &'a HBotData) -> Box<dyn FnMut(&ShipData) -> bool + 'a> {
        def_and_filter!(next_faction: faction => next_hull_type);
        def_and_filter!(next_hull_type: hull_type => next_rarity);
        def_and_filter!(next_rarity: rarity => finish);

        fn finish<'a>(f: &ViewFilterInfo, data: &'a HBotData, mut base: impl FnMut(&ShipData) -> bool + 'a) -> Box<dyn FnMut(&ShipData) -> bool + 'a> {
            match f.has_augment {
                Some(filter) => {
                    let azur_lane = data.azur_lane();
                    Box::new(move |s| base(s) && azur_lane.augment_by_ship_id(s.group_id).is_some() == filter)
                }
                None => Box::new(base)
            }
        }

        next_faction(self, data, |_| true)
    }
}
