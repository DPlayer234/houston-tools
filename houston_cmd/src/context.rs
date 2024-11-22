use std::sync::atomic::AtomicBool;

use serenity::gateway::client::Context as SerenityContext;
use serenity::http::Http;
use serenity::model::application::{CommandInteraction, ResolvedOption};
use serenity::model::id::{ChannelId, GuildId};
use serenity::model::user::User;

use crate::reply::CreateReply;
use crate::ReplyHandle;

#[derive(Debug)]
pub struct Context<'a> {
    pub(crate) reply_state: &'a AtomicBool,
    pub serenity: &'a SerenityContext,
    pub interaction: &'a CommandInteraction,
    pub(crate) options: &'a [ResolvedOption<'a>],
}

impl Copy for Context<'_> {}
impl Clone for Context<'_> {
    fn clone(&self) -> Self {
        *self
    }
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

    pub fn http(self) -> &'a Http {
        &self.serenity.http
    }

    pub fn user(self) -> &'a User {
        &self.interaction.user
    }

    pub fn channel_id(self) -> ChannelId {
        self.interaction.channel_id
    }

    pub fn guild_id(self) -> Option<GuildId> {
        self.interaction.guild_id
    }

    pub fn options(self) -> &'a [ResolvedOption<'a>] {
        self.options
    }

    pub async fn defer(self, ephemeral: bool) -> serenity::Result<()> {
        crate::reply::defer(self, ephemeral).await
    }

    pub async fn send(self, reply: CreateReply<'_>) -> serenity::Result<ReplyHandle<'a>> {
        crate::reply::send_reply(self, reply).await
    }
}
