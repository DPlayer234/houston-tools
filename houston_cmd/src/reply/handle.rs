use serenity::http::Http;
use serenity::model::prelude::*;

use crate::context::Context;

use super::CreateReply;

/// Represents a handle to a sent interaction response or follow-up,
/// allowing edits or deletion.
#[derive(Debug, Clone, Copy)]
pub struct ReplyHandle<'a> {
    http: &'a Http,
    interaction: &'a CommandInteraction,
    target: Target,
}

#[derive(Debug, Clone, Copy)]
enum Target {
    Original,
    Followup(MessageId)
}

impl<'a> ReplyHandle<'a> {
    pub(crate) fn original(ctx: Context<'a>) -> Self {
        Self {
            http: ctx.http(),
            interaction: ctx.interaction,
            target: Target::Original,
        }
    }

    pub(crate) fn followup(ctx: Context<'a>, message_id: MessageId) -> Self {
        Self {
            http: ctx.http(),
            interaction: ctx.interaction,
            target: Target::Followup(message_id),
        }
    }

    /// Delete the message.
    pub async fn delete(self) -> serenity::Result<()> {
        match self.target {
            Target::Original => self.interaction.delete_response(self.http).await?,
            Target::Followup(message_id) => self.interaction.delete_followup(self.http, message_id).await?,
        }

        Ok(())
    }

    /// Edit the message.
    ///
    /// You cannot edit whether a message is ephemeral.
    pub async fn edit(self, reply: CreateReply<'_>) -> serenity::Result<()> {
        match self.target {
            Target::Original => {
                let reply = reply.into_interaction_edit();
                self.interaction.edit_response(self.http, reply).await?;
            },
            Target::Followup(message_id) => {
                let reply = reply.into_interaction_followup();
                self.interaction.edit_followup(self.http, message_id, reply).await?;
            },
        }

        Ok(())
    }
}
