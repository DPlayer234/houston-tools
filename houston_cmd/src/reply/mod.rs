use std::sync::atomic::Ordering;

pub use create::CreateReply;
pub use edit::EditReply;
pub use handle::ReplyHandle;
use serenity::builder::*;

use crate::context::Context;

mod create;
mod edit;
mod handle;

pub async fn defer(ctx: Context<'_>, ephemeral: bool) -> serenity::Result<()> {
    let state = ctx.reply_state.load(Ordering::Relaxed);

    if state == 0 {
        let reply = CreateInteractionResponse::Defer(
            CreateInteractionResponseMessage::new().ephemeral(ephemeral),
        );

        ctx.interaction.create_response(ctx.http(), reply).await?;
        ctx.reply_state.store(1, Ordering::Relaxed);
    }

    Ok(())
}

pub async fn send_reply<'ctx>(
    ctx: Context<'ctx>,
    reply: CreateReply<'_>,
) -> serenity::Result<ReplyHandle<'ctx>> {
    let state = ctx.reply_state.load(Ordering::Relaxed);

    let handle = match state {
        0 => {
            let reply = reply.into_interaction_response();
            let reply = CreateInteractionResponse::Message(reply);
            ctx.interaction.create_response(ctx.http(), reply).await?;
            ctx.reply_state.store(2, Ordering::Relaxed);
            ReplyHandle::original(ctx)
        },
        1 => {
            let reply = reply.into_interaction_edit();
            ctx.interaction.edit_response(ctx.http(), reply).await?;
            ReplyHandle::original(ctx)
        },
        _ => {
            let reply = reply.into_interaction_followup();
            let message = ctx.interaction.create_followup(ctx.http(), reply).await?;
            ReplyHandle::followup(ctx, message.id)
        },
    };

    Ok(handle)
}
