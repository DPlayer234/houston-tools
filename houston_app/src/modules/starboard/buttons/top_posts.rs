use bson::Document;
use bson_model::Sort::Desc;
use utils::text::WriteStr as _;

use crate::buttons::prelude::*;
use crate::fmt::StringExt as _;
use crate::helper::discord::IdBytes;
use crate::modules::core::buttons::ToPage;
use crate::modules::starboard::{BoardId, get_board, model};

// View the post leaderboards.
#[derive(Debug, Clone, Serialize, Deserialize, ConstBuilder)]
pub struct View {
    #[serde(with = "As::<IdBytes>")]
    pub guild: GuildId,
    pub board: BoardId,
    #[builder(default = 0)]
    pub page: u16,
    #[serde(with = "As::<Option<IdBytes>>")]
    #[builder(default = None)]
    pub by_user: Option<UserId>,
}

impl View {
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

        while let Some(item) = cursor.try_next().await? {
            if index >= u64::from(PAGE_SIZE) {
                break;
            }

            index += 1;

            let rank = offset + index;
            let link = item.message.link(item.channel, Some(self.guild));
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

        let label = format!("### {} Top Posts", board.emoji());
        let description = description.or_default("<None>");

        let mut components = CreateComponents::new();

        components.push(CreateTextDisplay::new(label));

        if let Some(by_user) = self.by_user {
            components.push(CreateTextDisplay::new(format!("By: {}", by_user.mention())));
        }

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

        let reply = self.create_reply(ctx.data_ref()).await?;
        ctx.edit(reply.into()).await?;
        Ok(())
    }

    async fn modal_reply(mut self, ctx: ModalContext<'_>) -> Result {
        ctx.acknowledge().await?;

        self.page = ToPage::get_page(ctx.interaction())?;
        let reply = self.create_reply(ctx.data_ref()).await?;
        ctx.edit(reply.into()).await?;
        Ok(())
    }
}
