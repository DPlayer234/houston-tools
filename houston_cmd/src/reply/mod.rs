use std::sync::atomic::Ordering;

use serenity::builder::*;

pub use builder::CreateReply;
pub use handle::ReplyHandle;

use crate::context::Context;

mod builder;
mod handle;

pub async fn defer(
    ctx: Context<'_>,
    ephemeral: bool,
) -> serenity::Result<()> {
    let has_sent = ctx.reply_state.load(Ordering::Relaxed);

    if !has_sent {
        let reply = CreateInteractionResponse::Defer(
            CreateInteractionResponseMessage::new()
                .ephemeral(ephemeral)
        );

        ctx.interaction.create_response(ctx.http(), reply).await?;
        ctx.reply_state.store(true, Ordering::Relaxed);
    }

    Ok(())
}

pub async fn send_reply<'ctx>(
    ctx: Context<'ctx>,
    reply: CreateReply<'_>,
) -> serenity::Result<ReplyHandle<'ctx>> {
    let has_sent = ctx.reply_state.load(Ordering::Relaxed);

    let handle = if has_sent {
        let reply = reply.into_interaction_followup();
        let message = ctx.interaction.create_followup(ctx.http(), reply).await?;
        ReplyHandle::followup(ctx, message.id)
    } else {
        let reply = reply.into_interaction_response();
        let reply = CreateInteractionResponse::Message(reply);
        ctx.interaction.create_response(ctx.http(), reply).await?;
        ctx.reply_state.store(true, Ordering::Relaxed);
        ReplyHandle::original(ctx)
    };

    Ok(handle)
}
