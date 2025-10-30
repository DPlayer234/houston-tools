use bson_model::Sort::Desc;
use houston_utils::StringExt as _;
use houston_utils::discord::IdBytes;
use utils::text::WriteStr as _;

use crate::buttons::prelude::*;
use crate::modules::core::buttons::ToPage;
use crate::modules::starboard::{BoardId, get_board, model};

// View the leaderboards.
#[derive(Debug, Clone, Serialize, Deserialize, ConstBuilder)]
pub struct View {
    #[serde(with = "As::<IdBytes>")]
    pub guild: GuildId,
    pub board: BoardId,
    #[builder(default = 0)]
    pub page: u16,
}

impl View {
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
            writeln!(
                description,
                "{}. {}: {} {} from {} post(s)",
                offset + index,
                item.user.mention(),
                item.score,
                board.emoji(),
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

        let label = format!("### {} Leaderboards", board.emoji());
        let description = description.or_default("<None>");

        let mut components = CreateComponents::new();

        components.push(CreateTextDisplay::new(label));
        components.push(CreateSeparator::new(true));
        components.push(CreateTextDisplay::new(description));

        let pagination = ToPage::build_row(&mut self, |s| &mut s.page)
            .auto_page_count(page_count, has_more, MAX_PAGE);

        if let Some(nav) = pagination.end() {
            components.push(CreateSeparator::new(true));
            components.push(nav);
        }

        let container = CreateContainer::new(components).accent_color(data.config().embed_color);

        Ok(CreateReply::new()
            .components_v2(components![container])
            .allowed_mentions(CreateAllowedMentions::new()))
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
