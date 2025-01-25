use azur_lane::secretary::*;
use utils::text::truncate;
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
    pub name: Option<String>,
}

const PAGE_SIZE: usize = 15;

impl View {
    pub fn new(filter: Filter) -> Self {
        Self { page: 0, filter }
    }

    pub fn create_with_iter<'a>(
        mut self,
        data: &'a HBotData,
        mut iter: impl Iterator<Item = &'a SpecialSecretary>,
    ) -> Result<CreateReply<'a>> {
        let mut desc = String::new();
        let mut options = Vec::new();

        for sectretary in iter.by_ref().take(PAGE_SIZE) {
            writeln_str!(desc, "- **{}**", sectretary.name);

            let view_chat =
                super::special_secretary::View::new(sectretary.id).back(self.to_custom_data());
            options.push(CreateSelectMenuOption::new(
                truncate(sectretary.name.as_str(), 100),
                view_chat.to_custom_id(),
            ));
        }

        let rows = super::pagination!(self, options, iter, "View lines...");

        let author = CreateEmbedAuthor::new("Special Secretaries");

        let embed = CreateEmbed::new()
            .author(author)
            .description(desc)
            .color(data.config().embed_color);

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
    fn iterate<'a>(
        &self,
        data: &'a HAzurLane,
    ) -> Box<dyn Iterator<Item = &'a SpecialSecretary> + 'a> {
        match &self.name {
            Some(name) => self.apply_filter(data, data.special_secretaries_by_prefix(name)),
            None => self.apply_filter(data, data.special_secretaries().iter()),
        }
    }

    fn apply_filter<'a, I>(
        &self,
        _data: &'a HAzurLane,
        iter: I,
    ) -> Box<dyn Iterator<Item = &'a SpecialSecretary> + 'a>
    where
        I: Iterator<Item = &'a SpecialSecretary> + 'a,
    {
        Box::new(iter)
    }
}
