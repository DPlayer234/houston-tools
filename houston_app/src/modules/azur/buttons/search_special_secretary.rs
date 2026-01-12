use azur_lane::secretary::*;

use super::search::{All, Filtered, PAGE_SIZE};
use crate::buttons::prelude::*;
use crate::modules::azur::GameData;
use crate::modules::core::buttons::ToPage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View<'v> {
    page: u16,
    #[serde(borrow)]
    filter: Filter<'v>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Filter<'v> {
    pub name: Option<&'v str>,
}

impl<'v> View<'v> {
    pub fn new(filter: Filter<'v>) -> Self {
        Self { page: 0, filter }
    }

    fn create_with_iter<'a>(
        mut self,
        data: &'a HBotData,
        mut iter: Query<'a>,
    ) -> Result<CreateReply<'a>> {
        let page_iter = super::page_iter!(iter, self.page);
        let mut components = ComponentVec::new();

        components.push(CreateTextDisplay::new("### Special Secretaries"));
        components.push(CreateSeparator::new(true));

        for secretary in page_iter {
            let view_chat = super::special_secretary::View::builder()
                .secretary_id(secretary.id)
                .back(self.to_nav())
                .build();

            let button = CreateButton::new(view_chat.to_custom_id())
                .label(&secretary.name)
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
        self.create_with_iter(data, filtered)
    }
}

button_value!(for<'v> View<'v>, 22);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        let data = ctx.data_ref();
        let create = self.create(data)?;
        ctx.edit(create.into()).await?;
        Ok(())
    }

    async fn modal_reply(mut self, ctx: ModalContext<'_>) -> Result {
        self.page = ToPage::get_page(ctx.interaction)?;
        let data = ctx.data_ref();
        let create = self.create(data)?;
        ctx.edit(create.into()).await?;
        Ok(())
    }
}

type Query<'a> = Filtered<'a, SpecialSecretary, All>;

impl Filter<'_> {
    fn iterate(self, azur: &GameData) -> Query<'_> {
        match self.name {
            Some(name) => Filtered::by_prefix(azur.special_secretaries_by_prefix(name), All),
            None => Filtered::slice(azur.special_secretaries(), All),
        }
    }
}
