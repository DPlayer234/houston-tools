use azur_lane::secretary::*;
use utils::text::{WriteStr as _, truncate};

use super::acknowledge_unloaded;
use super::search::{All, Filtered, PAGE_SIZE};
use crate::buttons::prelude::*;
use crate::modules::azur::GameData;
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
        let mut desc = String::new();
        let mut options = Vec::new();

        for secretary in iter.by_ref().take(PAGE_SIZE) {
            writeln!(desc, "- **{}**", secretary.name);

            let view_chat = super::special_secretary::View::new(secretary.id).back(self.to_nav());
            options.push(CreateSelectMenuOption::new(
                truncate(secretary.name.as_str(), 100),
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
        let azur = data.config().azur()?;
        let filtered = self.filter.iterate(azur.game_data()).at_page(self.page);
        self.create_with_iter(data, filtered)
    }
}

button_value!(View<'_>, 22);
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

type Query<'a> = Filtered<'a, SpecialSecretary, All>;

impl Filter<'_> {
    fn iterate(self, azur: &GameData) -> Query<'_> {
        match self.name {
            Some(name) => Filtered::by_prefix(azur.special_secretaries_by_prefix(name), All),
            None => Filtered::slice(azur.special_secretaries(), All),
        }
    }
}
