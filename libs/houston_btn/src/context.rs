use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

use houston_cmd::{CreateReply, EditReply};
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal};
use serenity::http::Http;
use serenity::model::application::{ComponentInteraction, ModalInteraction};
use serenity::model::channel::GenericInteractionChannel;
use serenity::model::id::{GuildId, InteractionId};
use serenity::model::user::User;
use serenity::prelude::*;

use crate::Result;

pub struct InnerContext<'a, I: ?Sized> {
    pub state: &'a crate::EventHandler,
    pub serenity: &'a Context,
    pub interaction: &'a I,
    pub reply_state: AtomicBool,
}

impl<'a, I: ?Sized> InnerContext<'a, I> {
    pub fn new(state: &'a crate::EventHandler, serenity: &'a Context, interaction: &'a I) -> Self {
        Self {
            state,
            interaction,
            serenity,
            reply_state: AtomicBool::new(false),
        }
    }
}

impl<'a, I: AnyInteraction + 'a> InnerContext<'a, I> {
    pub fn unsize(self) -> InnerContext<'a, dyn AnyInteraction + 'a> {
        InnerContext {
            state: self.state,
            reply_state: self.reply_state,
            serenity: self.serenity,
            interaction: self.interaction,
        }
    }
}

/// Execution context for [`ButtonReply`](super::ButtonReply).
pub struct AnyContext<'a, I: ?Sized> {
    pub(super) inner: &'a InnerContext<'a, I>,
}

impl<I: fmt::Debug> fmt::Debug for AnyContext<'_, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnyContext")
            .field("interaction", self.inner.interaction)
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
    #[inline]
    fn try_first(&self) -> bool {
        !self.inner.reply_state.swap(true, Ordering::AcqRel)
    }

    /// Gets the HTTP client.
    pub fn http(&self) -> &'a Http {
        &self.inner.serenity.http
    }

    /// The source serenity context.
    pub fn serenity(&self) -> &'a Context {
        self.inner.serenity
    }

    /// The source interaction.
    pub fn interaction(&self) -> &'a I {
        self.inner.interaction
    }

    /// Acknowledges the interaction, expecting a later [`Self::edit`].
    ///
    /// # Errors
    ///
    /// Returns `Err` if acknowledging the interaction failed.
    pub async fn acknowledge(&self) -> Result {
        if self.try_first() {
            let interaction = self.interaction();
            CreateInteractionResponse::Acknowledge
                .execute(self.http(), interaction.id(), interaction.token())
                .await?;
        }

        Ok(())
    }

    /// Defers the interaction with a new message.
    ///
    /// # Errors
    ///
    /// Returns `Err` if deferring the interaction failed.
    pub async fn defer(&self, ephemeral: bool) -> Result {
        if self.try_first() {
            let interaction = self.interaction();
            let reply = CreateInteractionResponseMessage::new().ephemeral(ephemeral);
            CreateInteractionResponse::Defer(reply)
                .execute(self.http(), interaction.id(), interaction.token())
                .await?;
        }

        Ok(())
    }

    /// Replies to the interaction with a new message.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the reply is invalid or failed otherwise.
    pub async fn reply(&self, create: CreateReply<'_>) -> Result {
        if self.try_first() {
            let interaction = self.interaction();
            let reply = create.into_interaction_response();
            CreateInteractionResponse::Message(reply)
                .execute(self.http(), interaction.id(), interaction.token())
                .await?;
        } else {
            let interaction = self.interaction();
            create
                .into_interaction_followup()
                .execute(self.http(), None, interaction.token())
                .await?;
        }

        Ok(())
    }

    /// Edits a previous reply to the interaction or the original message.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the edit is invalid or failed otherwise.
    pub async fn edit(&self, edit: EditReply<'_>) -> Result {
        if self.try_first() {
            let interaction = self.interaction();
            edit.execute_as_response(self.http(), interaction.id(), interaction.token())
                .await?;
        } else {
            let interaction = self.interaction();
            edit.into_interaction_edit()
                .execute(self.http(), interaction.token())
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
    ///
    /// # Errors
    ///
    /// Returns `Err` if the modal is invalid or failed otherwise.
    pub async fn modal(&self, modal: CreateModal<'_>) -> Result {
        // this is only available for button interactions
        // because you cannot respond to a modal with another one
        let first = self.try_first();
        anyhow::ensure!(first, "cannot send modals after initial response");

        let interaction = self.interaction();
        CreateInteractionResponse::Modal(modal)
            .execute(self.http(), interaction.id, &interaction.token)
            .await?;
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
