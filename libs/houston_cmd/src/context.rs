use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::gateway::client::Context as SerenityContext;
use serenity::http::Http;
use serenity::model::application::{CommandInteraction, ResolvedValue};
use serenity::model::guild::Member;
use serenity::model::id::{GenericChannelId, GuildId};
use serenity::model::user::User;

use crate::ReplyHandle;
use crate::args::ResolvedOption;
use crate::reply::CreateReply;

const UNSENT: usize = 0;
const DEFER: usize = 1;
const SENT: usize = 2;

/// The context for a command invocation.
#[derive(Clone, Copy)]
pub struct Context<'a> {
    /// The serenity context that triggered this command.
    pub serenity: &'a SerenityContext,
    /// The command interaction that this context corresponds to.
    pub interaction: &'a CommandInteraction,
    /// Additional internal state.
    pub(crate) inner: &'a ContextInner<'a>,
}

impl fmt::Debug for Context<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("interaction", self.interaction)
            .field("inner", self.inner)
            .finish_non_exhaustive()
    }
}

/// Crate internal state for the context.
///
/// Present to avoid bloating the inline-size of the context struct. Plus, the
/// `reply_state` field needs to be held by reference anyways, so the only extra
/// indirection caused by this is for `options`.
#[derive(Debug)]
pub struct ContextInner<'a> {
    pub reply_state: AtomicUsize,
    pub options: Box<[ResolvedOption<'a>]>,
}

impl<'a> ContextInner<'a> {
    pub fn with_options(options: Box<[ResolvedOption<'a>]>) -> Self {
        Self {
            reply_state: AtomicUsize::new(UNSENT),
            options,
        }
    }

    pub fn empty() -> Self {
        Self::with_options(Box::default())
    }
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        serenity: &'a SerenityContext,
        interaction: &'a CommandInteraction,
        inner: &'a ContextInner<'a>,
    ) -> Self {
        Self {
            serenity,
            interaction,
            inner,
        }
    }

    /// Gets the HTTP client.
    pub fn http(self) -> &'a Http {
        &self.serenity.http
    }

    /// Gets the invoking user.
    pub fn user(self) -> &'a User {
        &self.interaction.user
    }

    /// Gets the invoking member.
    ///
    /// This is only present if invoked in guilds.
    pub fn member(self) -> Option<&'a Member> {
        self.interaction.member.as_deref()
    }

    /// Gets the ID of the channel the command was invoked in.
    pub fn channel_id(self) -> GenericChannelId {
        self.interaction.channel_id
    }

    /// Gets the ID of the guild the command was invoked in.
    pub fn guild_id(self) -> Option<GuildId> {
        self.interaction.guild_id
    }

    /// Gets the resolved options.
    pub fn options(self) -> &'a [ResolvedOption<'a>] {
        &self.inner.options
    }

    /// Gets the resolved value for an option by its name.
    ///
    /// If no option with that name was specified, returns [`None`].
    #[inline]
    pub fn option_value(self, name: &str) -> Option<&'a ResolvedValue<'a>> {
        self.inner
            .options
            .iter()
            .find(move |o| o.name == name)
            .map(|o| &o.value)
    }

    #[inline]
    fn get_reply_state(&self) -> usize {
        self.inner.reply_state.load(Ordering::Acquire)
    }

    #[inline]
    fn set_reply_state(&self, to: usize) {
        self.inner.reply_state.store(to, Ordering::Release);
    }

    /// Defers the response, specifying whether it is ephemeral.
    #[expect(clippy::missing_errors_doc)]
    pub async fn defer(self, ephemeral: bool) -> serenity::Result<()> {
        let state = self.get_reply_state();
        if state == UNSENT {
            let reply = CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(ephemeral),
            );

            self.interaction.create_response(self.http(), reply).await?;
            self.set_reply_state(DEFER);
        }

        Ok(())
    }

    /// Sends a reply.
    ///
    /// This automatically tracks whether this should be the initial response or
    /// a follow-up. However, don't mix use of this function with manual use
    /// of the interaction.
    #[expect(clippy::missing_errors_doc)]
    pub async fn send(self, reply: CreateReply<'_>) -> serenity::Result<ReplyHandle<'a>> {
        let state = self.get_reply_state();
        let target = match state {
            UNSENT => {
                let reply = reply.into_interaction_response();
                let reply = CreateInteractionResponse::Message(reply);
                self.interaction.create_response(self.http(), reply).await?;
                self.set_reply_state(SENT);
                None
            },
            DEFER => {
                let reply = reply.into_interaction_edit();
                self.interaction.edit_response(self.http(), reply).await?;
                self.set_reply_state(SENT);
                None
            },
            _ => {
                debug_assert!(state == SENT, "must be SENT state otherwise");
                let reply = reply.into_interaction_followup();
                let message = self.interaction.create_followup(self.http(), reply).await?;
                Some(message.id)
            },
        };

        Ok(ReplyHandle::new(
            self.http(),
            &self.interaction.token,
            target,
        ))
    }
}
