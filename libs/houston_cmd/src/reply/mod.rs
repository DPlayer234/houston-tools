use std::sync::atomic::{AtomicUsize, Ordering};

use serenity::builder::*;

use crate::context::Context;

mod create;
mod edit;
mod handle;

pub use create::CreateReply;
pub use edit::EditReply;
pub use handle::ReplyHandle;

pub const UNSENT: usize = 0;
const DEFER: usize = 1;
const SENT: usize = 2;

#[inline]
fn unsent_to_defer(state: &AtomicUsize) -> bool {
    state
        .compare_exchange(UNSENT, DEFER, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

pub async fn defer(ctx: Context<'_>, ephemeral: bool) -> serenity::Result<()> {
    if unsent_to_defer(&ctx.inner.reply_state) {
        let reply = CreateInteractionResponse::Defer(
            CreateInteractionResponseMessage::new().ephemeral(ephemeral),
        );

        ctx.interaction.create_response(ctx.http(), reply).await?;
    }

    Ok(())
}

pub async fn send_reply<'ctx>(
    ctx: Context<'ctx>,
    reply: CreateReply<'_>,
) -> serenity::Result<ReplyHandle<'ctx>> {
    let state = ctx.inner.reply_state.swap(SENT, Ordering::AcqRel);

    let handle = match state {
        UNSENT => {
            let reply = reply.into_interaction_response();
            let reply = CreateInteractionResponse::Message(reply);
            ctx.interaction.create_response(ctx.http(), reply).await?;
            ReplyHandle::original(ctx)
        },
        DEFER => {
            EditReply::from(reply)
                .execute_as_original_edit(ctx.http(), &ctx.interaction.token)
                .await?;
            ReplyHandle::original(ctx)
        },
        _ => {
            debug_assert!(state == SENT, "must be SENT state otherwise");
            let reply = reply.into_interaction_followup();
            let message = ctx.interaction.create_followup(ctx.http(), reply).await?;
            ReplyHandle::followup(ctx, message.id)
        },
    };

    Ok(handle)
}
