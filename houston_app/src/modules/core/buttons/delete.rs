use crate::buttons::prelude::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Delete;

button_value!(Delete, 14);
impl ButtonReply for Delete {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        ctx.acknowledge().await?;
        ctx.interaction.delete_response(&ctx.serenity.http).await?;
        Ok(())
    }
}
