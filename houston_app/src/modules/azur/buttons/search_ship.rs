use azur_lane::Faction;
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
    pub faction: Option<Faction>,
    pub hull_type: Option<HullType>,
    pub rarity: Option<ShipRarity>,
    pub has_augment: Option<bool>,
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
        mut iter: impl Iterator<Item = &'a ShipData>,
    ) -> Result<CreateReply<'a>> {
        let mut desc = String::new();
        let mut options = Vec::new();

        for ship in iter.by_ref().take(PAGE_SIZE) {
            let emoji = super::hull_emoji(ship.hull_type, data);

            writeln_str!(
                desc,
                "- {emoji} **{}** [{} {} {}]",
                ship.name,
                ship.rarity.name(),
                ship.faction.prefix().unwrap_or("Col."),
                ship.hull_type.designation(),
            );

            let view_ship = super::ship::View::new(ship.group_id).back(self.as_custom_data());
            options.push(
                CreateSelectMenuOption::new(&ship.name, view_ship.to_custom_id())
                    .emoji(emoji.clone()),
            );
        }

        let rows = super::pagination!(self, options, iter, "View ship...");

        let wiki_url = &*azur.wiki_urls().ship_list;
        let author = CreateEmbedAuthor::new("Ships").url(wiki_url);

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

impl ButtonArgsReply for View<'_> {
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

impl Filter<'_> {
    fn iterate<'a>(&self, azur: &'a GameData) -> Box<dyn Iterator<Item = &'a ShipData> + 'a> {
        match self.name {
            Some(name) => self.apply_filter(azur, azur.ships_by_prefix(name)),
            None => self.apply_filter(azur, azur.ships().iter()),
        }
    }

    fn apply_filter<'a, I>(
        &self,
        azur: &'a GameData,
        iter: I,
    ) -> Box<dyn Iterator<Item = &'a ShipData> + 'a>
    where
        I: Iterator<Item = &'a ShipData> + 'a,
    {
        macro_rules! def_and_filter {
            ($fn_name:ident: $field:ident => $next:ident) => {
                fn $fn_name<'a>(
                    f: &Filter<'_>,
                    azur: &'a GameData,
                    iter: impl Iterator<Item = &'a ShipData> + 'a,
                ) -> Box<dyn Iterator<Item = &'a ShipData> + 'a> {
                    match f.$field {
                        Some(filter) => $next(f, azur, iter.filter(move |s| s.$field == filter)),
                        None => $next(f, azur, iter),
                    }
                }
            };
        }

        def_and_filter!(next_faction: faction => next_hull_type);
        def_and_filter!(next_hull_type: hull_type => next_rarity);
        def_and_filter!(next_rarity: rarity => next_has_augment);

        fn next_has_augment<'a>(
            f: &Filter<'_>,
            azur: &'a GameData,
            iter: impl Iterator<Item = &'a ShipData> + 'a,
        ) -> Box<dyn Iterator<Item = &'a ShipData> + 'a> {
            match f.has_augment {
                Some(filter) => Box::new(iter.filter(move |s| {
                    azur.augments_by_ship_id(s.group_id).next().is_some() == filter
                })),
                None => Box::new(iter),
            }
        }

        next_faction(self, azur, iter)
    }
}
