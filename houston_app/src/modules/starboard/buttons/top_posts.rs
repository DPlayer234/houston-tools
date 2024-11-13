use bson::doc;
use serenity::futures::TryStreamExt;

use utils::text::write_str::*;

use crate::helper::bson_id;
use crate::helper::discord::get_pagination_buttons;
use crate::modules::starboard::get_board;
use crate::modules::starboard::model;
use crate::buttons::prelude::*;

// View the post leaderboards.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub guild: GuildId,
    pub board: ChannelId,
    pub page: u16,
}

impl View {
    pub fn new(guild: GuildId, board: ChannelId) -> Self {
        Self {
            guild,
            board,
            page: 0,
        }
    }

    pub async fn create_reply<'new>(mut self, data: &HBotData) -> anyhow::Result<CreateReply<'new>> {
        const PAGE_SIZE: u32 = 15;

        let db = data.database()?;
        let board = get_board(data.config(), self.guild, self.board)?;

        let filter = doc! {
            "board": bson_id!(self.board),
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

        if description.is_empty() {
            "<None>".clone_into(&mut description);
        }

        let embed = CreateEmbed::new()
            .title(format!("<#{}> Top Posts", self.board))
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
    async fn reply(self, ctx: ButtonContext<'_>) -> HResult {
        ctx.reply(CreateInteractionResponse::Acknowledge).await?;

        let reply = self.create_reply(ctx.data).await?;
        let edit = reply.to_slash_initial_response_edit(Default::default());

        ctx.edit_reply(edit).await?;
        Ok(())
    }
}
