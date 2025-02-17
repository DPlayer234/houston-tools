use std::sync::atomic::Ordering;

pub use create::CreateReply;
pub use edit::EditReply;
pub use handle::ReplyHandle;
use serenity::builder::*;

use crate::context::Context;

mod create;
mod edit;
mod handle;

pub const UNSENT: usize = 0;
const DEFER: usize = 1;
const SENT: usize = 2;

pub async fn defer(ctx: Context<'_>, ephemeral: bool) -> serenity::Result<()> {
    let state = ctx.inner.reply_state.load(Ordering::Relaxed);

    if state == UNSENT {
        let reply = CreateInteractionResponse::Defer(
            CreateInteractionResponseMessage::new().ephemeral(ephemeral),
        );

        ctx.interaction.create_response(ctx.http(), reply).await?;
        ctx.inner.reply_state.store(DEFER, Ordering::Relaxed);
    }

    Ok(())
}

pub async fn send_reply<'ctx>(
    ctx: Context<'ctx>,
    reply: CreateReply<'_>,
) -> serenity::Result<ReplyHandle<'ctx>> {
    let state = ctx.inner.reply_state.load(Ordering::Relaxed);

    let handle = match state {
        UNSENT => {
            let reply = reply.into_interaction_response();
            let reply = CreateInteractionResponse::Message(reply);
            ctx.interaction.create_response(ctx.http(), reply).await?;
            ctx.inner.reply_state.store(SENT, Ordering::Relaxed);
            ReplyHandle::original(ctx)
        },
        DEFER => {
            let reply = reply.into_interaction_edit();
            ctx.interaction.edit_response(ctx.http(), reply).await?;
            ctx.inner.reply_state.store(SENT, Ordering::Relaxed);
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
