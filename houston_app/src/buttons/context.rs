use std::sync::atomic::{AtomicBool, Ordering};

use serenity::http::Http;
use serenity::prelude::*;

use crate::prelude::*;

/// Execution context for [`ButtonArgsReply`](super::ButtonArgsReply).
#[derive(Debug, Clone)]
pub struct AnyContext<'a, I> {
    pub(super) reply_state: &'a AtomicBool,
    /// The serenity context.
    pub serenity: &'a Context,
    /// The source interaction.
    pub interaction: &'a I,
    /// The bot data.
    pub data: &'a HBotData,
}

/// Execution context for button interactions.
pub type ButtonContext<'a> = AnyContext<'a, ComponentInteraction>;

/// Execution context for modal interactions.
pub type ModalContext<'a> = AnyContext<'a, ModalInteraction>;

impl<I: AnyInteraction> AnyContext<'_, I> {
    /// Acknowledges the interaction, expecting a later [`Self::edit`].
    pub async fn acknowledge(&self) -> Result {
        let has_sent = self.reply_state.load(Ordering::Relaxed);

        if !has_sent {
            let reply = CreateInteractionResponse::Acknowledge;
            self.interaction
                .create_response(&self.serenity.http, reply)
                .await?;
            self.reply_state.store(true, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Defers the interaction with a new message.
    #[allow(dead_code, reason = "i might use this later")]
    pub async fn defer_as(&self, ephemeral: impl IntoEphemeral) -> Result {
        let has_sent = self.reply_state.load(Ordering::Relaxed);

        if !has_sent {
            let reply = CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(ephemeral.into_ephemeral()),
            );

            self.interaction
                .create_response(&self.serenity.http, reply)
                .await?;
            self.reply_state.store(true, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Replies to the interaction with a new message.
    #[allow(dead_code, reason = "i might use this later")]
    pub async fn reply(&self, create: CreateReply<'_>) -> Result {
        let has_sent = self.reply_state.load(Ordering::Relaxed);

        if has_sent {
            let reply = create.into_interaction_followup();
            self.interaction
                .create_followup(&self.serenity.http, reply)
                .await?;
        } else {
            let reply = create.into_interaction_response();
            let reply = CreateInteractionResponse::Message(reply);
            self.interaction
                .create_response(&self.serenity.http, reply)
                .await?;
            self.reply_state.store(true, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Edits a previous reply to the interaction or the original message.
    pub async fn edit(&self, edit: EditReply<'_>) -> Result {
        let has_sent = self.reply_state.load(Ordering::Relaxed);

        if has_sent {
            let reply = edit.into_interaction_edit();
            self.interaction
                .edit_response(&self.serenity.http, reply)
                .await?;
        } else {
            edit.execute_as_response(
                &self.serenity.http,
                self.interaction.id(),
                self.interaction.token(),
            )
            .await?;
            self.reply_state.store(true, Ordering::Relaxed);
        }

        Ok(())
    }
}

impl ButtonContext<'_> {
    /// Opens a modal for the user.
    ///
    /// This must be the first response and you cannot defer or acknowledge
    /// before this is called.
    pub async fn modal(&self, modal: CreateModal<'_>) -> Result {
        // this is only available for button interactions
        // because you cannot respond to a modal with another one
        let has_sent = self.reply_state.load(Ordering::Relaxed);
        anyhow::ensure!(!has_sent, "cannot send modals after initial response");

        let reply = CreateInteractionResponse::Modal(modal);
        self.interaction
            .create_response(&self.serenity.http, reply)
            .await?;
        self.reply_state.store(true, Ordering::Relaxed);
        Ok(())
    }
}

/// Represents any interaction struct. Used to allow code sharing between
/// different [`AnyContext`] instatiations.
pub trait AnyInteraction {
    fn id(&self) -> InteractionId;
    fn token(&self) -> &str;

    async fn create_response(
        &self,
        http: &Http,
        response: CreateInteractionResponse<'_>,
    ) -> serenity::Result<()>;

    async fn create_followup(
        &self,
        http: &Http,
        response: CreateInteractionResponseFollowup<'_>,
    ) -> serenity::Result<Message>;

    async fn edit_response(
        &self,
        http: &Http,
        edit: EditInteractionResponse<'_>,
    ) -> serenity::Result<Message>;
}

macro_rules! interaction_impl {
    ($($Interaction:ty)*) => { $(
        impl AnyInteraction for $Interaction {
            fn id(&self) -> InteractionId {
                self.id
            }

            fn token(&self) -> &str {
                &self.token
            }

            async fn create_response(
                &self,
                http: &Http,
                response: CreateInteractionResponse<'_>,
            ) -> serenity::Result<()> {
                self.create_response(http, response).await
            }

            async fn create_followup(
                &self,
                http: &Http,
                response: CreateInteractionResponseFollowup<'_>,
            ) -> serenity::Result<Message> {
                self.create_followup(http, response).await
            }

            async fn edit_response(
                &self,
                http: &Http,
                edit: EditInteractionResponse<'_>,
            ) -> serenity::Result<Message> {
                self.edit_response(http, edit).await
            }
        }
    )* };
}

interaction_impl!(ComponentInteraction ModalInteraction);
