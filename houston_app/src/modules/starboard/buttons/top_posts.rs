use bson::Document;
use bson_model::Sort::Desc;
use utils::text::WriteStr as _;

use crate::buttons::prelude::*;
use crate::fmt::StringExt as _;
use crate::fmt::discord::MessageLink;
use crate::helper::discord::{CreateComponents, id_as_u64, option_id_as_u64};
use crate::modules::core::buttons::ToPage;
use crate::modules::starboard::{BoardId, get_board, model};

// View the post leaderboards.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View {
    #[serde(with = "id_as_u64")]
    pub guild: GuildId,
    pub board: BoardId,
    pub page: u16,
    #[serde(with = "option_id_as_u64")]
    pub by_user: Option<UserId>,
}

impl View {
    pub fn new(guild: GuildId, board: BoardId, by_user: Option<UserId>) -> Self {
        Self {
            guild,
            board,
            page: 0,
            by_user,
        }
    }

    pub async fn create_reply<'new>(mut self, data: &HBotData) -> Result<CreateReply<'new>> {
        const PAGE_SIZE: u32 = 15;
        const MAX_PAGE: u16 = 50;

        let db = data.database()?;
        let board = get_board(data.config(), self.guild, self.board)?;

        let filter = self.message_filter()?;

        let sort = model::Message::sort()
            .max_reacts(Desc)
            .message(Desc)
            .into_document();

        let offset = u64::from(PAGE_SIZE) * u64::from(self.page);
        let mut cursor = model::Message::collection(db)
            .find(filter)
            .sort(sort)
            .limit((PAGE_SIZE + 1).into())
            .skip(offset)
            .await?;

        let mut description = String::new();
        let mut index = 0u64;

        if let Some(by_user) = self.by_user {
            writeln!(description, "-# By: {}", by_user.mention());
        }

        while let Some(item) = cursor.try_next().await? {
            if index >= u64::from(PAGE_SIZE) {
                break;
            }

            index += 1;

            let rank = offset + index;
            let link = MessageLink::new(self.guild, item.channel, item.message);
            let max_reacts = item.max_reacts;
            let emoji = board.emoji();

            if self.by_user.is_some() {
                writeln!(description, "{rank}. {link}: {max_reacts} {emoji}");
            } else {
                writeln!(
                    description,
                    "{rank}. {link} by {}: {max_reacts} {emoji}",
                    item.user.mention(),
                );
            }
        }

        if self.page > 0 && description.is_empty() {
            return Err(HArgError::new("No data for this page.").into());
        }

        let has_more = index >= u64::from(PAGE_SIZE);
        let page_count = if has_more {
            let filter = self.message_filter()?;

            model::Message::collection(db)
                .count_documents(filter)
                .limit((u64::from(MAX_PAGE) + 1) * u64::from(PAGE_SIZE))
                .await?
                .div_ceil(PAGE_SIZE.into())
                .try_into()?
        } else {
            self.page + 1
        };

        if self.by_user.is_some() && index == 0 {
            debug_assert!(!description.is_empty(), "by-user case always has content");
            writeln!(description, "<None>");
        }

        let description = description.or_default("<None>");

        let embed = CreateEmbed::new()
            .title(format!("{} Top Posts", board.emoji()))
            .color(data.config().embed_color)
            .description(description);

        let components = CreateComponents::from_iter(
            ToPage::build_row(&mut self, |s| &mut s.page)
                .auto_page_count(page_count, has_more, MAX_PAGE)
                .end(),
        );

        let reply = CreateReply::new().embed(embed).components(components);

        Ok(reply)
    }

    fn message_filter(&self) -> Result<Document> {
        let mut filter = model::Message::filter().board(self.board);
        if let Some(user) = self.by_user {
            filter = filter.user(user);
        }

        Ok(filter.into_document()?)
    }
}

button_value!(View, 12);
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
