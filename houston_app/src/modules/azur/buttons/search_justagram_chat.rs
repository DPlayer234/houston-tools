use azur_lane::juustagram::*;
use utils::text::write_str::*;

use crate::buttons::prelude::*;
use crate::modules::azur::data::HAzurLane;
use crate::modules::core::buttons::ToPage;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    page: u16,
    filter: Filter,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Filter {
    pub ship: Option<u32>,
}

const PAGE_SIZE: usize = 15;

impl View {
    pub fn new(filter: Filter) -> Self {
        Self { page: 0, filter }
    }

    pub fn create_with_iter<'a>(
        mut self,
        data: &'a HBotData,
        mut iter: impl Iterator<Item = &'a Chat>,
    ) -> Result<CreateReply<'a>> {
        let mut desc = String::new();
        let mut options = Vec::new();

        for chat in iter.by_ref().take(PAGE_SIZE) {
            if let Some(ship) = data.azur_lane().ship_by_id(chat.group_id) {
                writeln_str!(desc, "- **{}** [{}]", chat.name, ship.name,);
            } else {
                writeln_str!(desc, "- **{}**", chat.name,);
            }

            let view_chat =
                super::juustagram_chat::View::new(chat.chat_id).back(self.to_custom_data());
            options.push(CreateSelectMenuOption::new(
                &chat.name,
                view_chat.to_custom_id(),
            ));
        }

        super::pagination!(rows => self, options, iter);

        let author = CreateEmbedAuthor::new("JUUS [Chats]");

        let embed = CreateEmbed::new()
            .author(author)
            .description(desc)
            .color(data.config().embed_color);

        rows.push(super::create_string_select_menu_row(
            self.to_custom_id(),
            options,
            "Read chat...",
        ));

        Ok(CreateReply::new().embed(embed).components(rows))
    }

    pub fn create(self, data: &HBotData) -> Result<CreateReply<'_>> {
        let filtered = self
            .filter
            .iterate(data.azur_lane())
            .skip(PAGE_SIZE * usize::from(self.page));

        self.create_with_iter(data, filtered)
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

impl Filter {
    fn iterate<'a>(&self, data: &'a HAzurLane) -> Box<dyn Iterator<Item = &'a Chat> + 'a> {
        match &self.ship {
            Some(id) => self.apply_filter(data, data.juustagram_chats_by_ship_id(*id)),
            None => self.apply_filter(data, data.juustagram_chats().iter()),
        }
    }

    fn apply_filter<'a, I>(
        &self,
        _data: &'a HAzurLane,
        iter: I,
    ) -> Box<dyn Iterator<Item = &'a Chat> + 'a>
    where
        I: Iterator<Item = &'a Chat> + 'a,
    {
        Box::new(iter)
    }
}