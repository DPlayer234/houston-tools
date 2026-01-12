use azur_lane::juustagram::*;
use utils::text::truncate;

use super::search::{All, Filtered, PAGE_SIZE};
use crate::buttons::prelude::*;
use crate::modules::azur::{GameData, LoadedConfig};
use crate::modules::core::buttons::ToPage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View {
    page: u16,
    filter: Filter,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Filter {
    pub ship: Option<u32>,
}

impl View {
    pub fn new(filter: Filter) -> Self {
        Self { page: 0, filter }
    }

    fn create_with_iter<'a>(
        mut self,
        data: &'a HBotData,
        azur: LoadedConfig<'a>,
        mut iter: Query<'a>,
    ) -> Result<CreateReply<'a>> {
        let page_iter = super::page_iter!(iter, self.page);
        let mut components = ComponentVec::new();

        components.push(CreateTextDisplay::new("### JUUS [Chats]"));
        components.push(CreateSeparator::new(true));

        for chat in page_iter {
            let label = match azur.game_data().ship_by_id(chat.group_id) {
                Some(ship) => Cow::Owned(format!("{} [{}]", chat.name, ship.name)),
                None => Cow::Borrowed(chat.name.as_str()),
            };

            let view_chat = super::juustagram_chat::View::builder()
                .chat_id(chat.chat_id)
                .back(self.to_nav())
                .build();

            let button = CreateButton::new(view_chat.to_custom_id())
                .label(truncate(label, 80))
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

button_value!(View, 17);
impl ButtonReply for View {
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

type Query<'a> = Filtered<'a, Chat, All>;

impl Filter {
    fn iterate(self, azur: &GameData) -> Query<'_> {
        match self.ship {
            Some(id) => Filtered::by_lookup(azur.juustagram_chats_by_ship_id(id), All),
            None => Filtered::slice(azur.juustagram_chats(), All),
        }
    }
}
