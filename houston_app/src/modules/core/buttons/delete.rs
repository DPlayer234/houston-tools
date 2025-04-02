use crate::buttons::prelude::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Delete;

impl ButtonArgsReply for Delete {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        ctx.acknowledge().await?;
        ctx.interaction.delete_response(&ctx.serenity.http).await?;
        Ok(())
    }
}
