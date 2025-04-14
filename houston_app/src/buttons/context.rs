use std::sync::atomic::{AtomicBool, Ordering};

use serenity::http::Http;
use serenity::prelude::*;

use crate::prelude::*;

/// Execution context for [`ButtonReply`](super::ButtonReply).
#[derive(Clone)]
pub struct AnyContext<'a, I> {
    pub(super) reply_state: &'a AtomicBool,
    /// The serenity context.
    pub serenity: &'a Context,
    /// The source interaction.
    pub interaction: &'a I,
    /// The bot data.
    pub data: &'a HBotData,
}

utils::impl_debug!(for[I: AnyInteraction + std::fmt::Debug] struct AnyContext<'_, I>: { reply_state, interaction, data, .. });

/// Execution context for button interactions.
pub type ButtonContext<'a> = AnyContext<'a, ComponentInteraction>;

/// Execution context for modal interactions.
pub type ModalContext<'a> = AnyContext<'a, ModalInteraction>;

impl<I: AnyInteraction> AnyContext<'_, I> {
    #[inline]
    fn try_first(&self) -> bool {
        !self.reply_state.swap(true, Ordering::AcqRel)
    }

    /// Acknowledges the interaction, expecting a later [`Self::edit`].
    pub async fn acknowledge(&self) -> Result {
        if self.try_first() {
            let reply = CreateInteractionResponse::Acknowledge;
            self.interaction
                .create_response(&self.serenity.http, reply)
                .await?;
        }

        Ok(())
    }

    /// Defers the interaction with a new message.
    #[expect(dead_code, reason = "i might use this later")]
    pub async fn defer_as(&self, ephemeral: impl IntoEphemeral) -> Result {
        if self.try_first() {
            let reply = CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(ephemeral.into_ephemeral()),
            );

            self.interaction
                .create_response(&self.serenity.http, reply)
                .await?;
        }

        Ok(())
    }

    /// Replies to the interaction with a new message.
    #[expect(dead_code, reason = "i might use this later")]
    pub async fn reply(&self, create: CreateReply<'_>) -> Result {
        if self.try_first() {
            let reply = create.into_interaction_response();
            let reply = CreateInteractionResponse::Message(reply);
            self.interaction
                .create_response(&self.serenity.http, reply)
                .await?;
        } else {
            let reply = create.into_interaction_followup();
            self.interaction
                .create_followup(&self.serenity.http, reply)
                .await?;
        }

        Ok(())
    }

    /// Edits a previous reply to the interaction or the original message.
    pub async fn edit(&self, edit: EditReply<'_>) -> Result {
        if self.try_first() {
            edit.execute_as_response(
                &self.serenity.http,
                self.interaction.id(),
                self.interaction.token(),
            )
            .await?;
        } else {
            let reply = edit.into_interaction_edit();
            self.interaction
                .edit_response(&self.serenity.http, reply)
                .await?;
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
        let first = self.try_first();
        anyhow::ensure!(first, "cannot send modals after initial response");

        let reply = CreateInteractionResponse::Modal(modal);
        self.interaction
            .create_response(&self.serenity.http, reply)
            .await?;
        Ok(())
    }
}

/// Represents any interaction struct. Used to allow code sharing between
/// different [`AnyContext`] instatiations.
pub trait AnyInteraction {
    fn id(&self) -> InteractionId;
    fn token(&self) -> &str;
    fn guild_id(&self) -> Option<GuildId>;
    fn channel(&self) -> Option<&GenericInteractionChannel>;
    fn user(&self) -> &User;

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

            fn guild_id(&self) -> Option<GuildId> {
                self.guild_id
            }

            fn channel(&self) -> Option<&GenericInteractionChannel> {
                self.channel.as_ref()
            }

            fn user(&self) -> &User {
                &self.user
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
