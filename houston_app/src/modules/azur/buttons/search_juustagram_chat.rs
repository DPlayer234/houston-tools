use azur_lane::juustagram::*;
use utils::text::{WriteStr as _, truncate};

use super::acknowledge_unloaded;
use super::search::{All, Filtered, PAGE_SIZE};
use crate::buttons::prelude::*;
use crate::modules::azur::{GameData, LoadedConfig};
use crate::modules::core::buttons::ToPage;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View {
    page: u16,
    filter: Filter,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
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
        let mut desc = String::new();
        let mut options = Vec::new();

        for chat in iter.by_ref().take(PAGE_SIZE) {
            let chat_name: Cow<'_, str>;
            if let Some(ship) = azur.game_data().ship_by_id(chat.group_id) {
                writeln!(desc, "- **{}** [{}]", chat.name, ship.name);
                chat_name = format!("{} [{}]", chat.name, ship.name).into();
            } else {
                writeln!(desc, "- **{}**", chat.name);
                chat_name = chat.name.as_str().into();
            }

            let view_chat = super::juustagram_chat::View::new(chat.chat_id).back(self.to_nav());
            options.push(
                CreateSelectMenuOption::new(truncate(chat_name, 100), view_chat.to_custom_id())
                    .description(truncate(&chat.unlock_desc, 100)),
            );
        }

        let rows = super::pagination!(self, options, iter, "Read chat...");

        let author = CreateEmbedAuthor::new("JUUS [Chats]");

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

button_value!(View, 17);
impl ButtonReply for View {
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

type Query<'a> = Filtered<'a, Chat, All>;

impl Filter {
    fn iterate(self, azur: &GameData) -> Query<'_> {
        match self.ship {
            Some(id) => Filtered::by_lookup(azur.juustagram_chats_by_ship_id(id), All),
            None => Filtered::slice(azur.juustagram_chats(), All),
        }
    }
}
