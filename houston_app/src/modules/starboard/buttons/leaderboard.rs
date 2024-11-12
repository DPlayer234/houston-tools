use crate::buttons::prelude::*;

// View the leaderboards.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub board: ChannelId,
    pub page: u16,
    pub ephemeral: bool,
}

#[cfg(not(feature = "db"))]
impl ButtonArgsReply for View {
    async fn reply(self, _ctx: ButtonContext<'_>) -> HResult {
        anyhow::bail!("starboard not supported with db");
    }
}

#[cfg(feature = "db")]
impl View {
    pub async fn create_reply<'new>(mut self, data: &HBotData) -> anyhow::Result<CreateReply<'new>> {
        use anyhow::Context;
        use mongodb::bson::doc;
        use serenity::futures::TryStreamExt;

        use utils::text::write_str::*;

        use crate::helper::bson_id;
        use crate::helper::discord::get_pagination_buttons;
        use crate::modules::starboard::model;

        const PAGE_SIZE: u32 = 15;

        let db = data.database()?;
        let board = data.config()
            .starboard
            .iter()
            .find(|b| b.channel == self.board)
            .context("starboard not found")?;

        let filter = doc! {
            "guild": bson_id!(board.guild),
            "board": bson_id!(board.channel),
        };

        let sort = doc! {
            "score": -1,
        };

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
                "{}. <@{}>: {} {}",
                offset + index, item.user, item.score, board.emoji,
            );
        }

        let embed = CreateEmbed::new()
            .title(format!("<#{}> Leaderboards", board.channel))
            .color(DEFAULT_EMBED_COLOR)
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

#[cfg(feature = "db")]
impl ButtonArgsReply for View {
    async fn reply(self, ctx: ButtonContext<'_>) -> HResult {
        ctx.reply(CreateInteractionResponse::Acknowledge).await?;

        let reply = self.create_reply(ctx.data).await?;
        let edit = reply.to_slash_initial_response_edit(Default::default());

        ctx.edit_reply(edit).await?;
        Ok(())
    }
}
