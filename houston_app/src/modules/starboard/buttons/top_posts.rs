use bson::doc;

use utils::text::write_str::*;

use crate::buttons::prelude::*;
use crate::helper::discord::{get_pagination_buttons, id_as_u64};
use crate::modules::starboard::get_board;
use crate::modules::starboard::model;
use crate::modules::starboard::BoardId;

// View the post leaderboards.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

        let db = data.database()?;
        let board = get_board(data.config(), self.guild, self.board)?;

        let filter = doc! {
            "board": self.board.get(),
        };

        let sort = doc! {
            "max_reacts": -1,
            "message": -1,
        };

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
            writeln_str!(
                description,
                "{}. https://discord.com/channels/{}/{}/{} by <@{}>: {} {}",
                offset + index, self.guild, item.channel, item.message, item.user, item.max_reacts, board.emoji.as_emoji(),
            );
        }

        let description = crate::fmt::written_or(description, "<None>");

        let embed = CreateEmbed::new()
            .title(format!("{} Top Posts", board.emoji))
            .color(data.config().embed_color)
            .description(description)
            .footer(CreateEmbedFooter::new(format!("Page {}", self.page + 1)));

        let components = get_pagination_buttons(&mut self, utils::field_mut!(Self: page), index >= u64::from(PAGE_SIZE))
            .as_slice()
            .to_vec();

        let reply = CreateReply::new()
            .embed(embed)
            .components(components);

        Ok(reply)
    }
}

impl ButtonArgsReply for View {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        ctx.reply(CreateInteractionResponse::Acknowledge).await?;

        let reply = self.create_reply(ctx.data).await?;
        let edit = reply.into_interaction_edit();

        ctx.edit_reply(edit).await?;
        Ok(())
    }
}
