use std::sync::atomic::AtomicBool;

use serenity::gateway::client::Context as SerenityContext;
use serenity::http::Http;
use serenity::model::prelude::*;

use crate::reply::CreateReply;
use crate::ReplyHandle;

/// The context for a command invocation.
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    pub(crate) reply_state: &'a AtomicBool,
    /// The serenity context that triggered this command.
    pub serenity: &'a SerenityContext,
    /// The command interaction that this context corresponds to.
    pub interaction: &'a CommandInteraction,
    pub(crate) options: &'a [ResolvedOption<'a>],
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        reply_state: &'a AtomicBool,
        serenity: &'a SerenityContext,
        interaction: &'a CommandInteraction,
    ) -> Self {
        Self {
            reply_state,
            serenity,
            interaction,
            options: &[],
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
    pub fn channel_id(self) -> ChannelId {
        self.interaction.channel_id
    }

    /// Gets the ID of the guild the command was invoked in.
    pub fn guild_id(self) -> Option<GuildId> {
        self.interaction.guild_id
    }

    /// Gets the resolved options.
    pub fn options(self) -> &'a [ResolvedOption<'a>] {
        self.options
    }

    /// Defers the response, specifying whether it is ephemeral.
    pub async fn defer(self, ephemeral: bool) -> serenity::Result<()> {
        crate::reply::defer(self, ephemeral).await
    }

    /// Sends a reply.
    ///
    /// This automatically tracks whether this should be the initial response or
    /// a follow-up. However, don't mix use of this function with manual use
    /// of the interaction.
    pub async fn send(self, reply: CreateReply<'_>) -> serenity::Result<ReplyHandle<'a>> {
        crate::reply::send_reply(self, reply).await
    }
}
