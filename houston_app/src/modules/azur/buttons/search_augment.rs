use azur_lane::equip::*;
use azur_lane::ship::*;
use utils::text::write_str::*;

use crate::buttons::prelude::*;
use crate::modules::azur::{Config, GameData};
use crate::modules::core::buttons::ToPage;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    page: u16,
    filter: Filter,
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

    fn create_with_iter<'a>(
        mut self,
        data: &'a HBotData,
        config: &'a Config,
        mut iter: impl Iterator<Item = &'a Augment>,
    ) -> Result<CreateReply<'a>> {
        let mut desc = String::new();
        let mut options = Vec::new();

        for augment in iter.by_ref().take(PAGE_SIZE) {
            writeln_str!(desc, "- **{}** [{}]", augment.name, augment.rarity.name());

            let view = super::augment::View::new(augment.augment_id).back(self.to_custom_data());
            options.push(CreateSelectMenuOption::new(
                &augment.name,
                view.to_custom_id(),
            ));
        }

        let rows = super::pagination!(self, options, iter, "View augment module...");

        let author = CreateEmbedAuthor::new("Augment Modules").url(&*config.augment_list_url);

        let embed = CreateEmbed::new()
            .author(author)
            .description(desc)
            .color(data.config().embed_color);

        Ok(CreateReply::new().embed(embed).components(rows))
    }

    pub fn create(self, data: &HBotData) -> Result<CreateReply<'_>> {
        let config = data.config().azur()?;
        let filtered = self
            .filter
            .iterate(config.game_data())
            .skip(PAGE_SIZE * usize::from(self.page));

        self.create_with_iter(data, config, filtered)
    }
}

impl ButtonMessage for View {
    fn edit_reply(self, ctx: ButtonContext<'_>) -> Result<EditReply<'_>> {
        self.create(ctx.data).map(EditReply::from)
    }

    fn edit_modal_reply(mut self, ctx: ModalContext<'_>) -> Result<EditReply<'_>> {
        ToPage::set_page_from(&mut self.page, ctx.interaction);
        self.create(ctx.data).map(EditReply::from)
    }
}

type FIter<'a> = Box<dyn Iterator<Item = &'a Augment> + 'a>;

impl Filter {
    fn iterate<'a>(&self, azur: &'a GameData) -> FIter<'a> {
        match &self.name {
            Some(name) => self.apply_filter(azur, azur.augments_by_prefix(name.as_str())),
            None => self.apply_filter(azur, azur.augments().iter()),
        }
    }

    fn apply_filter<'a, I>(&self, azur: &'a GameData, iter: I) -> FIter<'a>
    where
        I: Iterator<Item = &'a Augment> + 'a,
    {
        fn next_hull_type<'a>(
            f: &Filter,
            azur: &'a GameData,
            iter: impl Iterator<Item = &'a Augment> + 'a,
        ) -> FIter<'a> {
            match f.hull_type {
                Some(filter) => next_rarity(
                    f,
                    azur,
                    iter.filter(move |s| match &s.usability {
                        AugmentUsability::HullTypes(h) => h.contains(&filter),
                        AugmentUsability::UniqueShipId(id) => {
                            azur.ship_by_id(*id).is_some_and(|s| s.hull_type == filter)
                        },
                    }),
                ),
                None => next_rarity(f, azur, iter),
            }
        }

        fn next_rarity<'a>(
            f: &Filter,
            azur: &'a GameData,
            iter: impl Iterator<Item = &'a Augment> + 'a,
        ) -> FIter<'a> {
            match f.rarity {
                Some(filter) => {
                    next_unique_ship_id(f, azur, iter.filter(move |s| s.rarity == filter))
                },
                None => next_unique_ship_id(f, azur, iter),
            }
        }

        fn next_unique_ship_id<'a>(
            f: &Filter,
            _data: &'a GameData,
            iter: impl Iterator<Item = &'a Augment> + 'a,
        ) -> FIter<'a> {
            match f.unique_ship_id {
                Some(filter) => {
                    Box::new(iter.filter(move |s| s.usability.unique_ship_id() == Some(filter)))
                },
                None => Box::new(iter),
            }
        }

        next_hull_type(self, azur, iter)
    }
}
