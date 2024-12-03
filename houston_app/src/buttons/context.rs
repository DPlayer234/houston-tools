use std::sync::atomic::{AtomicBool, Ordering};

use serenity::prelude::*;

use crate::prelude::*;

macro_rules! declare_context {
    ($Name:ident, $Interaction:ty) => {
        /// Execution context for [`ButtonArgsReply`](super::ButtonArgsReply).
        #[derive(Debug, Clone)]
        pub struct $Name<'a> {
            pub(super) reply_state: &'a AtomicBool,
            /// The serenity context.
            pub serenity: &'a Context,
            /// The source interaction.
            pub interaction: &'a $Interaction,
            /// The bot data.
            pub data: &'a HBotData,
        }

        #[allow(dead_code, reason = "emits methods on multiple types")]
        impl $Name<'_> {
            /// Acknowledges the interaction, expecting a later [`Self::edit`].
            pub async fn acknowledge(&self) -> Result {
                let has_sent = self.reply_state.load(Ordering::Relaxed);

                if !has_sent {
                    let reply = CreateInteractionResponse::Acknowledge;
                    self.interaction.create_response(&self.serenity.http, reply).await?;
                    self.reply_state.store(true, Ordering::Relaxed);
                }

                Ok(())
            }

            /// Defers the interaction with a new message.
            pub async fn defer_as(&self, ephemeral: impl IntoEphemeral) -> Result {
                let has_sent = self.reply_state.load(Ordering::Relaxed);

                if !has_sent {
                    let reply = CreateInteractionResponse::Defer(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(ephemeral.into_ephemeral())
                    );
                    self.interaction.create_response(&self.serenity.http, reply).await?;
                    self.reply_state.store(true, Ordering::Relaxed);
                }

                Ok(())
            }

            /// Replies to the interaction with a new message.
            pub async fn reply(&self, create: CreateReply<'_>) -> Result {
                let has_sent = self.reply_state.load(Ordering::Relaxed);

                if has_sent {
                    let reply = create.into_interaction_followup();
                    self.interaction.create_followup(&self.serenity.http, reply).await?;
                } else {
                    let reply = create.into_interaction_response();
                    let reply = CreateInteractionResponse::Message(reply);
                    self.interaction.create_response(&self.serenity.http, reply).await?;
                    self.reply_state.store(true, Ordering::Relaxed);
                }

                Ok(())
            }

            /// Edits a previous reply to the interaction or the original message.
            pub async fn edit(&self, edit: EditReply<'_>) -> Result {
                let has_sent = self.reply_state.load(Ordering::Relaxed);

                if has_sent {
                    let reply = edit.into_interaction_edit();
                    self.interaction.edit_response(&self.serenity.http, reply).await?;
                } else {
                    edit.execute_as_response(&self.serenity.http, self.interaction.id, &self.interaction.token).await?;
                    self.reply_state.store(true, Ordering::Relaxed);
                }

                Ok(())
            }
        }
    };
}

declare_context!(ButtonContext, ComponentInteraction);
declare_context!(ModalContext, ModalInteraction);

impl ButtonContext<'_> {
    /// Opens a modal for the user.
    ///
    /// This must be the first response and you cannot defer or acknowledge before this.
    pub async fn modal(&self, modal: CreateModal<'_>) -> Result {
        let has_sent = self.reply_state.load(Ordering::Relaxed);
        anyhow::ensure!(!has_sent, "cannot send modals after initial response");

        let reply = CreateInteractionResponse::Modal(modal);
        self.interaction.create_response(&self.serenity.http, reply).await?;
        self.reply_state.store(true, Ordering::Relaxed);
        Ok(())
    }
}
