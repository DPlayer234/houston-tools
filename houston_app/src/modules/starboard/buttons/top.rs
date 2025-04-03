use bson_model::Sort::Desc;
use utils::text::write_str::*;

use crate::buttons::prelude::*;
use crate::helper::discord::id_as_u64;
use crate::modules::core::buttons::ToPage;
use crate::modules::starboard::{BoardId, get_board, model};

// View the leaderboards.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View {
    #[serde(with = "id_as_u64")]
    pub guild: GuildId,
    pub board: BoardId,
    pub page: u16,
}

impl View {
    pub fn new(guild: GuildId, board: BoardId) -> Self {
        Self {
            guild,
            board,
            page: 0,
        }
    }

    pub async fn create_reply<'new>(mut self, data: &HBotData) -> Result<CreateReply<'new>> {
        const PAGE_SIZE: u32 = 15;
        const MAX_PAGE: u16 = 50;

        let db = data.database()?;
        let board = get_board(data.config(), self.guild, self.board)?;

        let filter = model::Score::filter().board(self.board).into_document()?;

        let sort = model::Score::sort()
            .score(Desc)
            .post_count(Desc)
            .into_document();

        let offset = u64::from(PAGE_SIZE) * u64::from(self.page);
        let mut cursor = model::Score::collection(db)
            .find(filter)
            .sort(sort)
            .limit((PAGE_SIZE + 1).into())
            .skip(offset)
            .await?;

        let mut description = String::new();
        let mut index = 0u64;

        while let Some(item) = cursor.try_next().await? {
            if index >= u64::from(PAGE_SIZE) {
                break;
            }

            index += 1;
            writeln_str!(
                description,
                "{}. {}: {} {} from {} post(s)",
                offset + index,
                item.user.mention(),
                item.score,
                board.emoji.as_emoji(),
                item.post_count,
            );
        }

        if self.page > 0 && description.is_empty() {
            return Err(HArgError::new("No data for this page.").into());
        }

        let has_more = index >= u64::from(PAGE_SIZE);
        let page_count = if has_more {
            let filter = model::Score::filter().board(self.board).into_document()?;

            model::Score::collection(db)
                .count_documents(filter)
                .limit((u64::from(MAX_PAGE) + 1) * u64::from(PAGE_SIZE))
                .await?
                .div_ceil(PAGE_SIZE.into())
                .try_into()?
        } else {
            self.page + 1
        };

        let description = crate::fmt::written_or(description, "<None>");

        let embed = CreateEmbed::new()
            .title(format!("{} Leaderboards", board.emoji))
            .color(data.config().embed_color)
            .description(description);

        let components = Vec::from_iter(
            ToPage::build_row(&mut self, |s| &mut s.page)
                .auto_page_count(page_count, has_more, MAX_PAGE)
                .end(),
        );

        let reply = CreateReply::new().embed(embed).components(components);
        Ok(reply)
    }
}

button_value!(View, 11);
impl ButtonReply for View {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        ctx.acknowledge().await?;

        let reply = self.create_reply(ctx.data).await?;
        ctx.edit(reply.into()).await?;
        Ok(())
    }

    async fn modal_reply(mut self, ctx: ModalContext<'_>) -> Result {
        ctx.acknowledge().await?;

        self.page = ToPage::get_page(ctx.interaction)?;
        let reply = self.create_reply(ctx.data).await?;
        ctx.edit(reply.into()).await?;
        Ok(())
    }
}
