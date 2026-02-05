use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use houston_cmd::{CreateReply, EditReply, ReplyHandle};
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal};
use serenity::gateway::client::Context;
use serenity::http::Http;
use serenity::model::application::{ComponentInteraction, ModalInteraction};
use serenity::model::channel::GenericInteractionChannel;
use serenity::model::id::{GuildId, InteractionId, MessageId};
use serenity::model::user::User;

use crate::Result;

const UNSENT: usize = 0;
const DEFER: usize = 1;
const SENT: usize = 2;

pub struct ContextInner<'a> {
    pub state: &'a crate::EventHandler,
    pub reply_state: AtomicUsize,
}

impl<'a> ContextInner<'a> {
    pub fn new(state: &'a crate::EventHandler) -> Self {
        Self {
            state,
            reply_state: AtomicUsize::new(UNSENT),
        }
    }
}

/// Execution context for [`ButtonReply`](super::ButtonReply).
pub struct AnyContext<'a, I: ?Sized> {
    /// The serenity context that triggered this interaction.
    pub serenity: &'a Context,
    /// The source interaction that this context corresponds to.
    pub interaction: &'a I,
    pub(super) inner: &'a ContextInner<'a>,
}

impl<I: fmt::Debug> fmt::Debug for AnyContext<'_, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnyContext")
            .field("interaction", self.interaction)
            .finish_non_exhaustive()
    }
}

impl<I: ?Sized> Clone for AnyContext<'_, I> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<I: ?Sized> Copy for AnyContext<'_, I> {}

/// Execution context for button interactions.
pub type ButtonContext<'a> = AnyContext<'a, ComponentInteraction>;

/// Execution context for modal interactions.
pub type ModalContext<'a> = AnyContext<'a, ModalInteraction>;

/// Execution context for errors.
pub type ErrorContext<'a> = AnyContext<'a, dyn AnyInteraction + 'a>;

impl<'a, I: ?Sized + AnyInteraction> AnyContext<'a, I> {
    pub(crate) fn new(
        serenity: &'a Context,
        interaction: &'a I,
        inner: &'a ContextInner<'a>,
    ) -> Self {
        Self {
            serenity,
            interaction,
            inner,
        }
    }

    #[inline]
    fn get_reply_state(&self) -> usize {
        self.inner.reply_state.load(Ordering::Acquire)
    }

    #[inline]
    fn set_reply_state(&self, to: usize) {
        self.inner.reply_state.store(to, Ordering::Release);
    }

    fn reply_handle(&self, target: Option<MessageId>) -> ReplyHandle<'a> {
        ReplyHandle::new(self.http(), self.interaction.token(), target)
    }

    /// Gets the HTTP client.
    pub fn http(&self) -> &'a Http {
        &self.serenity.http
    }

    /// Acknowledges the interaction, expecting a later [`Self::edit`].
    ///
    /// This function does nothing if the interaction has already been responded
    /// to.
    ///
    /// # Errors
    ///
    /// Returns `Err` if acknowledging the interaction failed.
    pub async fn acknowledge(&self) -> Result {
        let state = self.get_reply_state();
        if state == UNSENT {
            CreateInteractionResponse::Acknowledge
                .execute(self.http(), self.interaction.id(), self.interaction.token())
                .await?;

            // - next `edit` needs to edit the original
            // - next `reply` needs to create a followup
            self.set_reply_state(SENT);
        } else {
            anyhow::ensure!(
                state != DEFER,
                "cannot acknowledge and defer the same context"
            );
        }

        Ok(())
    }

    /// Defers the interaction with a new message. This new message will become
    /// the original message for this interaction.
    ///
    /// This function does nothing if the interaction has already been responded
    /// to.
    ///
    /// # Errors
    ///
    /// Returns `Err` if deferring the interaction failed.
    pub async fn defer(&self, ephemeral: bool) -> Result {
        let state = self.get_reply_state();
        if state == UNSENT {
            let reply = CreateInteractionResponseMessage::new().ephemeral(ephemeral);
            CreateInteractionResponse::Defer(reply)
                .execute(self.http(), self.interaction.id(), self.interaction.token())
                .await?;
            self.set_reply_state(DEFER);
        }

        Ok(())
    }

    /// Replies to the interaction with a new message.
    ///
    /// If the interaction has not been responded to yet, this new message will
    /// become the original message for this interaction.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the reply is invalid or failed otherwise.
    pub async fn reply(&self, create: CreateReply<'_>) -> Result<ReplyHandle<'a>> {
        let state = self.get_reply_state();
        let target = match state {
            UNSENT => {
                let reply = create.into_interaction_response();
                CreateInteractionResponse::Message(reply)
                    .execute(self.http(), self.interaction.id(), self.interaction.token())
                    .await?;
                self.set_reply_state(SENT);
                None
            },
            DEFER => {
                create
                    .into_interaction_edit()
                    .execute(self.http(), self.interaction.token())
                    .await?;
                self.set_reply_state(SENT);
                None
            },
            _ => {
                debug_assert!(state == SENT, "must be SENT state otherwise");
                let message = create
                    .into_interaction_followup()
                    .execute(self.http(), None, self.interaction.token())
                    .await?;
                Some(message.id)
            },
        };

        Ok(self.reply_handle(target))
    }

    /// Edits the original message.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the edit is invalid or failed otherwise.
    pub async fn edit(&self, edit: EditReply<'_>) -> Result<ReplyHandle<'a>> {
        if self.get_reply_state() == UNSENT {
            edit.execute_as_response(self.http(), self.interaction.id(), self.interaction.token())
                .await?;
            self.set_reply_state(SENT);
        } else {
            edit.into_interaction_edit()
                .execute(self.http(), self.interaction.token())
                .await?;
        }

        Ok(self.reply_handle(None))
    }
}

impl ButtonContext<'_> {
    /// Opens a modal for the user.
    ///
    /// This must be the first response and you cannot defer or acknowledge
    /// before this is called.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the modal is invalid or failed otherwise.
    pub async fn modal(&self, modal: CreateModal<'_>) -> Result {
        // this is only available for button interactions
        // because you cannot respond to a modal with another one
        let first = self.get_reply_state() == UNSENT;
        anyhow::ensure!(first, "cannot send modals after initial response");

        CreateInteractionResponse::Modal(modal)
            .execute(self.http(), self.interaction.id, &self.interaction.token)
            .await?;
        self.set_reply_state(SENT);
        Ok(())
    }
}

pub trait Sealed {}

/// Represents any interaction struct. Used to allow code sharing between
/// different [`AnyContext`] instatiations.
///
/// This trait cannot be implemented by other crates.
pub trait AnyInteraction: Send + Sync + Sealed {
    fn id(&self) -> InteractionId;
    fn token(&self) -> &str;
    fn guild_id(&self) -> Option<GuildId>;
    fn channel(&self) -> Option<&GenericInteractionChannel>;
    fn user(&self) -> &User;
}

macro_rules! interaction_impl {
    ($($Interaction:ty)*) => { $(
        impl Sealed for $Interaction {}
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
        }
    )* };
}

interaction_impl!(ComponentInteraction ModalInteraction);
