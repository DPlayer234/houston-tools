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
    faction: Option<Faction>,
    hull_type: Option<HullType>,
    rarity: Option<ShipRarity>,
    has_augment: Option<bool>
}

impl ViewFilter {
    pub fn modify_with_iter<'a>(self, create: CreateReply, iter: impl Iterator<Item = &'a ShipData>) -> CreateReply {
        create
    }
}

impl ButtonArgsModify for ViewFilter {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        const PAGE_SIZE: usize = 15;

        let mut predicate = self.filter.predicate(data);
        let filtered = data.azur_lane().ship_list.iter()
            .filter(move |&s| predicate(s))
            .skip(PAGE_SIZE * usize::from(self.page)).take(PAGE_SIZE);

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
