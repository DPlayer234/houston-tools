use azur_lane::Faction;
use azur_lane::ship::*;

use super::search::{Filtered, Filtering, PAGE_SIZE};
use crate::buttons::prelude::*;
use crate::helper::discord::components::{CreateComponents, components, section_components};
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
    pub hull_type: Option<HullType>,
    pub rarity: Option<ShipRarity>,
    pub has_augment: Option<bool>,
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
        let page_iter = super::page_iter!(iter, self.page);
        let mut components = CreateComponents::new();

        components.push(CreateSection::new(
            section_components![CreateTextDisplay::new("### Ships")],
            CreateSectionAccessory::Button(
                CreateButton::new_link(&azur.wiki_urls().ship_list).label("Wiki"),
            ),
        ));

        components.push(CreateSeparator::new(true));

        for ship in page_iter {
            let emoji = super::hull_emoji(ship.hull_type, data);

            let view = super::ship::View::new(ship.group_id).back(self.to_nav());
            let button = CreateButton::new(view.to_custom_id())
                .label(&ship.name)
                .emoji(emoji.clone())
                .style(ButtonStyle::Secondary);

            components.push(CreateActionRow::buttons(vec![button]));
        }

        super::page_nav!(components, self, iter);

        Ok(CreateReply::new().components_v2(components![
            CreateContainer::new(components).accent_color(data.config().embed_color)
        ]))
    }

    pub fn create(self, data: &HBotData) -> Result<CreateReply<'_>> {
        let azur = data.config().azur()?;
        let filtered = self.filter.iterate(azur.game_data()).at_page(self.page);
        self.create_with_iter(data, azur, filtered)
    }
}

button_value!(for<'v> View<'v>, 5);
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

type Query<'a, 'v> = Filtered<'a, ShipData, (Filter<'v>, &'a GameData)>;

impl<'v> Filter<'v> {
    fn iterate(self, azur: &GameData) -> Query<'_, 'v> {
        let filter = (self, azur);
        match self.name {
            Some(name) => Filtered::by_prefix(azur.ships_by_prefix(name), filter),
            None => Filtered::slice(azur.ships(), filter),
        }
    }
}

impl Filtering<ShipData> for (Filter<'_>, &GameData) {
    fn is_match(&self, item: &ShipData) -> bool {
        let (filter, azur) = *self;
        let Filter {
            faction,
            rarity,
            hull_type,
            has_augment,
            ..
        } = filter;

        fn match_has_augment(azur: &GameData, item: &ShipData, has_augment: bool) -> bool {
            azur.augments_by_ship_id(item.group_id).next().is_some() == has_augment
        }

        faction.is_none_or(|f| item.faction == f)
            && hull_type.is_none_or(|h| item.hull_type == h)
            && rarity.is_none_or(|r| item.rarity == r)
            && has_augment.is_none_or(|h| match_has_augment(azur, item, h))
    }
}
