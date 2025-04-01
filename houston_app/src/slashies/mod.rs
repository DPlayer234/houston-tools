use args::SlashMember;
use houston_cmd::Context;

use crate::data::IntoEphemeral;
use crate::fmt::discord::DisplayCommand;
use crate::prelude::*;

pub mod args;

pub mod prelude {
    pub use bson_model::ModelDocument as _;
    pub use houston_cmd::{Context, chat_command, context_command, sub_command};

    pub use super::args::*;
    pub use super::{ContextExt as _, SlashUserExt as _, create_reply};
    pub use crate::prelude::*;
}

/// Pre-command execution hook.
pub async fn pre_command(ctx: Context<'_>) {
    log::info!(
        "{}: {}",
        ctx.user().name,
        DisplayCommand::new(&ctx.interaction.data, ctx.options()),
    );
}

/// Command execution error handler.
#[cold]
pub async fn error_handler(error: houston_cmd::Error<'_>) {
    match error {
        houston_cmd::Error::Command { error, ctx } => command_error(ctx, error).await,
        houston_cmd::Error::ArgInvalid { message, ctx } => {
            let msg = format!("Argument invalid: {}", message);
            context_error(ctx, msg.into()).await
        },
        houston_cmd::Error::ArgParse { error, input, ctx } => {
            let msg = format!("Argument invalid: {error}\nCaused by input: '{input}'");
            context_error(ctx, msg.into()).await
        },
        _ => log::error!("Oh noes, we got an error: {error:?}"),
    }

    async fn command_error(ctx: Context<'_>, err: anyhow::Error) {
        let message = match err.downcast::<HArgError>() {
            Ok(err) => err.msg,
            Err(err) => {
                if let Some(ser_err) = err.downcast_ref::<serenity::Error>() {
                    // print both errors to preserve the stack trace, if present
                    log::warn!("Discord error in command: {ser_err:?} / {err:?}")
                } else {
                    log::error!("Error in command: {err:?}");
                }

                format!("Internal error: ```{err}```").into()
            },
        };

        context_error(ctx, message).await
    }

    async fn context_error(ctx: Context<'_>, feedback: Cow<'_, str>) {
        let embed = CreateEmbed::new()
            .description(feedback)
            .color(ERROR_EMBED_COLOR);

        let reply = create_reply(Ephemeral).embed(embed);
        if let Err(err) = ctx.send(reply).await {
            log::error!("Error in error handler: {err:?}")
        };
    }
}

pub fn create_reply<'new>(ephemeral: impl IntoEphemeral) -> CreateReply<'new> {
    CreateReply::new().ephemeral(ephemeral.into_ephemeral())
}

/// Extension trait for the poise context.
pub trait ContextExt<'a> {
    async fn defer_as(self, ephemeral: impl IntoEphemeral) -> Result;

    #[must_use]
    fn data_ref(self) -> &'a HBotData;

    fn require_guild_id(self) -> Result<GuildId>;
}

impl<'a> ContextExt<'a> for Context<'a> {
    async fn defer_as(self, ephemeral: impl IntoEphemeral) -> Result {
        self.defer(ephemeral.into_ephemeral()).await?;
        Ok(())
    }

    fn data_ref(self) -> &'a HBotData {
        self.serenity.data_ref::<HContextData>()
    }

    fn require_guild_id(self) -> Result<GuildId> {
        self.guild_id().context("must be used in guild")
    }
}

pub trait SlashUserExt<'a>: Sized {
    type Inner;
    fn or_invoking(self, ctx: Context<'a>) -> Result<Self::Inner>;
}

impl<'a> SlashUserExt<'a> for Option<SlashMember<'a>> {
    type Inner = SlashMember<'a>;

    #[inline]
    fn or_invoking(self, ctx: Context<'a>) -> Result<Self::Inner> {
        match self {
            Some(member) => Ok(member),
            None => SlashMember::from_ctx(ctx),
        }
    }
}
