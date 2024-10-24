use azur_lane::equip::*;
use azur_lane::ship::*;
use utils::text::write_str::*;

use crate::buttons::*;

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
        View { page: 0, filter }
    }

    pub fn modify_with_iter<'a>(mut self, create: CreateReply, iter: impl Iterator<Item = &'a Augment>) -> CreateReply {
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

        if options.is_empty() {
            let embed = CreateEmbed::new()
                .color(ERROR_EMBED_COLOR)
                .description("No results for that filter.");

            return create.embed(embed);
        }

        let embed = CreateEmbed::new()
            .title("Augment Modules")
            .footer(CreateEmbedFooter::new(format!("Page {}", self.page + 1)))
            .description(desc)
            .color(DEFAULT_EMBED_COLOR);

        let options = CreateSelectMenuKind::String { options };
        let mut rows = vec![
            CreateActionRow::SelectMenu(CreateSelectMenu::new(self.to_custom_id(), options).placeholder("View augment module..."))
        ];

        if let Some(pagination) = super::get_pagination_buttons(&mut self, utils::field_mut!(Self: page), has_next) {
            rows.insert(0, pagination);
        }

        create.embed(embed).components(rows)
    }

    pub fn modify(self, data: &HBotData, create: CreateReply) -> CreateReply {
        let filtered = self.filter
            .iterate(data.azur_lane())
            .skip(PAGE_SIZE * usize::from(self.page));

        self.modify_with_iter(create, filtered)
    }
}

impl ButtonMessage for View {
    fn create_reply(self, ctx: ButtonContext<'_>) -> anyhow::Result<CreateReply> {
        Ok(self.modify(ctx.data, ctx.create_reply()))
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
                Some(filter) => next_rarity(f, data, iter.filter(move |s| s.usability.hull_types().is_some_and(|h| h.contains(&filter)))),
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
