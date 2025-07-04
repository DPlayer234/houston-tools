use azur_lane::equip::*;
use azur_lane::ship::*;

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
    pub hull_type: Option<HullType>,
    pub rarity: Option<AugmentRarity>,
    pub unique_ship_id: Option<u32>,
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
            section_components![CreateTextDisplay::new("### Augment Modules")],
            CreateSectionAccessory::Button(
                CreateButton::new_link(&azur.wiki_urls().augment_list).label("Wiki"),
            ),
        ));

        components.push(CreateSeparator::new(true));

        for augment in page_iter {
            let view = super::augment::View::new(augment.augment_id).back(self.to_nav());
            let button = CreateButton::new(view.to_custom_id())
                .label(&augment.name)
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

button_value!(for<'v> View<'v>, 9);
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

type Query<'a, 'v> = Filtered<'a, Augment, (Filter<'v>, &'a GameData)>;

impl<'v> Filter<'v> {
    fn iterate(self, azur: &GameData) -> Query<'_, 'v> {
        let filter = (self, azur);
        match self.name {
            Some(name) => Filtered::by_prefix(azur.augments_by_prefix(name), filter),
            None => Filtered::slice(azur.augments(), filter),
        }
    }
}

impl Filtering<Augment> for (Filter<'_>, &GameData) {
    fn is_match(&self, item: &Augment) -> bool {
        let (filter, azur) = *self;
        let Filter {
            rarity,
            unique_ship_id,
            hull_type,
            ..
        } = filter;

        fn match_hull_type(azur: &GameData, item: &Augment, hull_type: HullType) -> bool {
            match &item.usability {
                AugmentUsability::HullTypes(h) => h.contains(&hull_type),
                AugmentUsability::UniqueShipId(id) => azur
                    .ship_by_id(*id)
                    .is_some_and(|s| s.hull_type == hull_type),
            }
        }

        rarity.is_none_or(|r| item.rarity == r)
            && unique_ship_id.is_none_or(|i| item.usability.unique_ship_id() == Some(i))
            && hull_type.is_none_or(|h| match_hull_type(azur, item, h))
    }
}
