use serenity::prelude::*;

use crate::prelude::*;

macro_rules! declare_context {
    ($Name:ident, $Interaction:ty) => {
        /// Execution context for [`ButtonArgsReply`](super::ButtonArgsReply).
        #[derive(Debug, Clone)]
        pub struct $Name<'a> {
            /// The serenity context.
            pub serenity: &'a Context,
            /// The source interaction.
            pub interaction: &'a $Interaction,
            /// The bot data.
            pub data: &'a HBotData,
        }

        impl $Name<'_> {
            /// Replies to the interaction.
            pub async fn reply(&self, create: CreateInteractionResponse<'_>) -> Result {
                self.interaction.create_response(&self.serenity.http, create).await?;
                Ok(())
            }

            /// Edits a previous reply to the interaction.
            pub async fn edit_reply(&self, edit: EditInteractionResponse<'_>) -> Result {
                self.interaction.edit_response(&self.serenity.http, edit).await?;
                Ok(())
            }
        }
    };
}

declare_context!(ButtonContext, ComponentInteraction);
declare_context!(ModalContext, ModalInteraction);
