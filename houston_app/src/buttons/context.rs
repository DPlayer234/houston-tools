use serenity::prelude::*;

use crate::prelude::*;

/// Execution context for [`ButtonArgsReply`].
#[derive(Debug, Clone)]
pub struct ButtonContext<'a> {
    /// The serenity context.
    pub serenity: &'a Context,
    /// The source interaction.
    pub interaction: ButtonInteraction<'a>,
    /// The bot data.
    pub data: &'a HBotData,
}

impl ButtonContext<'_> {
    /// Replies to the interaction.
    pub async fn reply(&self, create: CreateInteractionResponse<'_>) -> Result {
        match self.interaction {
            ButtonInteraction::Component(i) => i.create_response(&self.serenity.http, create).await?,
            ButtonInteraction::Modal(i) => i.create_response(&self.serenity.http, create).await?,
        };
        Ok(())
    }

    /// Edits a previous reply to the interaction.
    pub async fn edit_reply(&self, edit: EditInteractionResponse<'_>) -> Result {
        match self.interaction {
            ButtonInteraction::Component(i) => i.edit_response(&self.serenity.http, edit).await?,
            ButtonInteraction::Modal(i) => i.edit_response(&self.serenity.http, edit).await?,
        };
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ButtonInteraction<'a> {
    Component(&'a ComponentInteraction),
    Modal(&'a ModalInteraction),
}

impl<'a> ButtonInteraction<'a> {
    pub fn id(self) -> InteractionId {
        match self {
            Self::Component(i) => i.id,
            Self::Modal(i) => i.id,
        }
    }

    pub fn token(self) -> &'a str {
        match self {
            Self::Component(i) => &i.token,
            Self::Modal(i) => &i.token,
        }
    }

    pub fn guild_id(self) -> Option<GuildId> {
        match self {
            Self::Component(i) => i.guild_id,
            Self::Modal(i) => i.guild_id,
        }
    }

    pub fn user(self) -> &'a User {
        match self {
            Self::Component(i) => &i.user,
            Self::Modal(i) => &i.user,
        }
    }

    pub fn message(self) -> Option<&'a Message> {
        match self {
            Self::Component(i) => Some(&*i.message),
            Self::Modal(i) => i.message.as_deref(),
        }
    }

    pub fn modal_data(self) -> Option<&'a ModalInteractionData> {
        match self {
            Self::Modal(i) => Some(&i.data),
            _ => None,
        }
    }
}
