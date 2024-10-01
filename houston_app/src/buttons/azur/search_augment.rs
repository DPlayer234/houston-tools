use std::fmt::Write;

use azur_lane::equip::*;
use azur_lane::ship::*;
use utils::Discard;

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

            writeln!(
                desc,
                "- **{}** [{}]",
                augment.name, augment.rarity.name(),
            ).discard();

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

impl Filter {
    fn iterate<'a>(&self, data: &'a HAzurLane) -> Box<dyn Iterator<Item = &'a Augment> + 'a> {
        let predicate = self.predicate(data);
        match &self.name {
            Some(name) => Box::new(data.augments_by_prefix(name.as_str()).filter(predicate)),
            None => Box::new(data.augments().iter().filter(predicate))
        }
    }

    fn predicate<'a>(&self, data: &'a HAzurLane) -> Box<dyn FnMut(&&Augment) -> bool + 'a> {
        fn next_hull_type<'a>(f: &Filter, data: &'a HAzurLane, mut base: impl FnMut(&&Augment) -> bool + 'a) -> Box<dyn FnMut(&&Augment) -> bool + 'a> {
            match f.hull_type {
                Some(filter) => next_rarity(f, data, move |s| base(s) && s.usability.hull_types().is_some_and(|h| h.contains(&filter))),
                None => next_rarity(f, data, base),
            }
        }

        fn next_rarity<'a>(f: &Filter, data: &'a HAzurLane, mut base: impl FnMut(&&Augment) -> bool + 'a) -> Box<dyn FnMut(&&Augment) -> bool + 'a> {
            match f.rarity {
                Some(filter) => next_unique_ship_id(f, data, move |s| base(s) && s.rarity == filter),
                None => next_unique_ship_id(f, data, base),
            }
        }

        fn next_unique_ship_id<'a>(f: &Filter, data: &'a HAzurLane, mut base: impl FnMut(&&Augment) -> bool + 'a) -> Box<dyn FnMut(&&Augment) -> bool + 'a> {
            match f.unique_ship_id {
                Some(filter) => finish(f, data, move |s| base(s) && s.usability.unique_ship_id() == Some(filter)),
                None => finish(f, data, base),
            }
        }

        fn finish<'a>(_f: &Filter, _data: &'a HAzurLane, base: impl FnMut(&&Augment) -> bool + 'a) -> Box<dyn FnMut(&&Augment) -> bool + 'a> {
            Box::new(base)
        }

        next_hull_type(self, data, |_| true)
    }
}
