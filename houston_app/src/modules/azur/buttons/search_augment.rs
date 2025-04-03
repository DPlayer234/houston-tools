use azur_lane::equip::*;
use azur_lane::ship::*;
use utils::text::write_str::*;

use super::acknowledge_unloaded;
use crate::buttons::prelude::*;
use crate::modules::azur::{GameData, LoadedConfig};
use crate::modules::core::buttons::ToPage;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View<'v> {
    page: u16,
    #[serde(borrow)]
    filter: Filter<'v>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Filter<'v> {
    pub name: Option<&'v str>,
    pub hull_type: Option<HullType>,
    pub rarity: Option<AugmentRarity>,
    pub unique_ship_id: Option<u32>,
}

const PAGE_SIZE: usize = 15;

impl<'v> View<'v> {
    pub fn new(filter: Filter<'v>) -> Self {
        Self { page: 0, filter }
    }

    fn create_with_iter<'a>(
        mut self,
        data: &'a HBotData,
        azur: LoadedConfig<'a>,
        mut iter: impl Iterator<Item = &'a Augment>,
    ) -> Result<CreateReply<'a>> {
        let mut desc = String::new();
        let mut options = Vec::new();

        for augment in iter.by_ref().take(PAGE_SIZE) {
            writeln_str!(desc, "- **{}** [{}]", augment.name, augment.rarity.name());

            let view = super::augment::View::new(augment.augment_id).back(self.to_nav());
            options.push(CreateSelectMenuOption::new(
                &augment.name,
                view.to_custom_id(),
            ));
        }

        let rows = super::pagination!(self, options, iter, "View augment module...");

        let wiki_url = &*azur.wiki_urls().augment_list;
        let author = CreateEmbedAuthor::new("Augment Modules").url(wiki_url);

        let embed = CreateEmbed::new()
            .author(author)
            .description(desc)
            .color(data.config().embed_color);

        Ok(CreateReply::new().embed(embed).components(rows))
    }

    pub fn create(self, data: &HBotData) -> Result<CreateReply<'_>> {
        let azur = data.config().azur()?;
        let filtered = self
            .filter
            .iterate(azur.game_data())
            .skip(PAGE_SIZE * usize::from(self.page));

        self.create_with_iter(data, azur, filtered)
    }
}

button_value!(View<'_>, 9);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        acknowledge_unloaded(&ctx).await?;
        let create = self.create(ctx.data)?;
        ctx.edit(create.into()).await
    }

    async fn modal_reply(mut self, ctx: ModalContext<'_>) -> Result {
        acknowledge_unloaded(&ctx).await?;
        self.page = ToPage::get_page(ctx.interaction)?;
        let create = self.create(ctx.data)?;
        ctx.edit(create.into()).await
    }
}

type FIter<'a> = Box<dyn Iterator<Item = &'a Augment> + 'a>;

impl Filter<'_> {
    fn iterate<'a>(&self, azur: &'a GameData) -> FIter<'a> {
        match self.name {
            Some(name) => self.apply_filter(azur, azur.augments_by_prefix(name)),
            None => self.apply_filter(azur, azur.augments().iter()),
        }
    }

    fn apply_filter<'a, I>(&self, azur: &'a GameData, iter: I) -> FIter<'a>
    where
        I: Iterator<Item = &'a Augment> + 'a,
    {
        fn next_hull_type<'a>(
            f: &Filter<'_>,
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
            f: &Filter<'_>,
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
            f: &Filter<'_>,
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
