use azur_lane::equip::*;
use azur_lane::ship::*;
use utils::text::write_str::*;

use crate::buttons::prelude::*;
use crate::modules::azur::data::HAzurLane;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    page: u16,
    filter: Filter
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Filter {
    pub name: Option<String>,
    pub hull_type: Option<HullType>,
    pub rarity: Option<AugmentRarity>,
    pub unique_ship_id: Option<u32>,
}

const PAGE_SIZE: usize = 15;

impl View {
    pub fn new(filter: Filter) -> Self {
        Self { page: 0, filter }
    }

    pub fn modify_with_iter<'a>(
        mut self,
        data: &'a HBotData,
        iter: impl Iterator<Item = &'a Augment>,
    ) -> CreateReply<'a> {
        let mut desc = String::new();
        let mut options = Vec::new();
        let mut has_next = false;

        for augment in iter {
            if options.len() >= PAGE_SIZE {
                has_next = true;
                break
            }

            writeln_str!(
                desc,
                "- **{}** [{}]",
                augment.name, augment.rarity.name(),
            );

            let view = super::augment::View::new(augment.augment_id).new_message();
            options.push(CreateSelectMenuOption::new(&augment.name, view.to_custom_id()));
        }

        let author = CreateEmbedAuthor::new("Augment Modules")
            .url(config::azur_lane::equip::AUGMENT_LIST_URL);

        if options.is_empty() {
            let embed = CreateEmbed::new()
                .author(author)
                .color(ERROR_EMBED_COLOR)
                .description("No results for that filter.");

            return CreateReply::new().embed(embed);
        }

        let embed = CreateEmbed::new()
            .author(author)
            .footer(CreateEmbedFooter::new(format!("Page {}", self.page + 1)))
            .description(desc)
            .color(data.config().embed_color);

        let mut rows = Vec::new();
        if let Some(pagination) = super::get_pagination_buttons(&mut self, utils::field_mut!(Self: page), has_next) {
            rows.push(pagination);
        }

        rows.push(super::create_string_select_menu_row(
            self.to_custom_id(),
            options,
            "View augment module...",
        ));

        CreateReply::new().embed(embed).components(rows)
    }

    pub fn modify(self, data: &HBotData) -> CreateReply<'_> {
        let filtered = self.filter
            .iterate(data.azur_lane())
            .skip(PAGE_SIZE * usize::from(self.page));

        self.modify_with_iter(data, filtered)
    }
}

impl ButtonMessage for View {
    fn create_reply(self, ctx: ButtonContext<'_>) -> anyhow::Result<CreateReply<'_>> {
        Ok(self.modify(ctx.data))
    }
}

type FIter<'a> = Box<dyn Iterator<Item = &'a Augment> + 'a>;

impl Filter {
    fn iterate<'a>(&self, data: &'a HAzurLane) -> FIter<'a> {
        match &self.name {
            Some(name) => self.apply_filter(data, data.augments_by_prefix(name.as_str())),
            None => self.apply_filter(data, data.augments().iter()),
        }
    }

    fn apply_filter<'a, I>(&self, data: &'a HAzurLane, iter: I) -> FIter<'a>
    where
        I: Iterator<Item = &'a Augment> + 'a,
    {
        fn next_hull_type<'a>(f: &Filter, data: &'a HAzurLane, iter: impl Iterator<Item = &'a Augment> + 'a) -> FIter<'a> {
            match f.hull_type {
                Some(filter) => next_rarity(f, data, iter.filter(move |s| match &s.usability {
                    AugmentUsability::HullTypes(h) => h.contains(&filter),
                    AugmentUsability::UniqueShipId(id) => data.ship_by_id(*id).is_some_and(|s| s.hull_type == filter),
                })),
                None => next_rarity(f, data, iter),
            }
        }

        fn next_rarity<'a>(f: &Filter, data: &'a HAzurLane, iter: impl Iterator<Item = &'a Augment> + 'a) -> FIter<'a> {
            match f.rarity {
                Some(filter) => next_unique_ship_id(f, data, iter.filter(move |s| s.rarity == filter)),
                None => next_unique_ship_id(f, data, iter),
            }
        }

        fn next_unique_ship_id<'a>(f: &Filter, _data: &'a HAzurLane, iter: impl Iterator<Item = &'a Augment> + 'a) -> FIter<'a>{
            match f.unique_ship_id {
                Some(filter) => Box::new(iter.filter(move |s| s.usability.unique_ship_id() == Some(filter))),
                None => Box::new(iter),
            }
        }

        next_hull_type(self, data, iter)
    }
}
