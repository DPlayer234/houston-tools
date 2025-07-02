use azur_lane::Faction;
use azur_lane::equip::*;
use utils::text::WriteStr as _;

use super::search::{Filtered, Filtering, PAGE_SIZE};
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

impl<'v> View<'v> {
    pub fn new(filter: Filter<'v>) -> Self {
        Self { page: 0, filter }
    }

    fn create_with_iter<'a>(
        mut self,
        data: &'a HBotData,
        azur: LoadedConfig<'a>,
        mut iter: Query<'a, 'v>,
    ) -> Result<CreateReply<'a>> {
        let mut desc = String::new();
        let mut options = Vec::new();

        for equip in iter.by_ref().take(PAGE_SIZE) {
            writeln!(
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
        let filtered = self.filter.iterate(azur.game_data()).at_page(self.page);
        self.create_with_iter(data, azur, filtered)
    }
}

button_value!(for<'v> View<'v>, 8);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        let create = self.create(ctx.data)?;
        ctx.edit(create.into()).await
    }

    async fn modal_reply(mut self, ctx: ModalContext<'_>) -> Result {
        self.page = ToPage::get_page(ctx.interaction)?;
        let create = self.create(ctx.data)?;
        ctx.edit(create.into()).await
    }
}

type Query<'a, 'v> = Filtered<'a, Equip, Filter<'v>>;

impl<'v> Filter<'v> {
    fn iterate(self, azur: &GameData) -> Query<'_, 'v> {
        match self.name {
            Some(name) => Filtered::by_prefix(azur.equips_by_prefix(name), self),
            None => Filtered::slice(azur.equips(), self),
        }
    }
}

impl Filtering<Equip> for Filter<'_> {
    fn is_match(&self, item: &Equip) -> bool {
        let Filter {
            faction,
            kind,
            rarity,
            ..
        } = *self;

        faction.is_none_or(|f| item.faction == f)
            && kind.is_none_or(|k| item.kind == k)
            && rarity.is_none_or(|r| item.rarity == r)
    }
}
