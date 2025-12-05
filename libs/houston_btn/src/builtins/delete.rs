use crate::prelude::*;

/// A button value that deletes the source message on interaction.
///
/// [`ButtonAction::key`](crate::ButtonAction::key) = [`u16::MAX - 1`](u16::MAX)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Delete;

button_value!(Delete, (u16::MAX - 1) as usize);
impl ButtonReply for Delete {
    async fn reply(self, ctx: ButtonContext<'_>) -> crate::Result {
        ctx.acknowledge().await?;
        ctx.interaction().delete_response(ctx.http()).await?;
        Ok(())
    }

    async fn modal_reply(self, ctx: ModalContext<'_>) -> crate::Result {
        ctx.acknowledge().await?;
        ctx.interaction().delete_response(ctx.http()).await?;
        Ok(())
    }
}
