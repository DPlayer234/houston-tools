use azur_lane::Faction;
use azur_lane::equip::*;
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

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Filter<'v> {
    pub name: Option<&'v str>,
    pub faction: Option<Faction>,
    pub kind: Option<EquipKind>,
    pub rarity: Option<EquipRarity>,
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
        mut iter: impl Iterator<Item = &'a Equip>,
    ) -> Result<CreateReply<'a>> {
        let mut desc = String::new();
        let mut options = Vec::new();

        for equip in iter.by_ref().take(PAGE_SIZE) {
            writeln_str!(
                desc,
                "- **{}** [{} {} {}]",
                equip.name,
                equip.rarity.name(),
                equip.faction.prefix().unwrap_or("Col."),
                equip.kind.name(),
            );

            let view_equip = super::equip::View::new(equip.equip_id).back(self.to_nav());
            options.push(CreateSelectMenuOption::new(
                &equip.name,
                view_equip.to_custom_id(),
            ));
        }

        let rows = super::pagination!(self, options, iter, "View equipment...");

        let wiki_url = &*azur.wiki_urls().equipment_list;
        let author = CreateEmbedAuthor::new("Equipments").url(wiki_url);

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

button_value!(View<'_>, 8);
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

type BoxIter<'a> = Box<dyn Iterator<Item = &'a Equip> + 'a>;

impl<'v> Filter<'v> {
    fn iterate<'a>(self, azur: &'a GameData) -> FilteredIter<'a, 'v> {
        let inner: BoxIter<'a> = match self.name {
            Some(name) => Box::new(azur.equips_by_prefix(name)),
            None => Box::new(azur.equips().iter()),
        };

        FilteredIter {
            inner,
            filter: self,
        }
    }
}

struct FilteredIter<'a, 'v> {
    inner: BoxIter<'a>,
    filter: Filter<'v>,
}

impl<'a> Iterator for FilteredIter<'a, '_> {
    type Item = &'a Equip;

    fn next(&mut self) -> Option<Self::Item> {
        let Filter {
            faction,
            kind,
            rarity,
            ..
        } = self.filter;

        loop {
            let item = self.inner.next()?;
            if faction.is_none_or(|f| item.faction == f)
                && kind.is_none_or(|k| item.kind == k)
                && rarity.is_none_or(|r| item.rarity == r)
            {
                return Some(item);
            }
        }
    }
}
