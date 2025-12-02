use houston_cmd::{BoxFuture, Context};

use crate::data::IntoEphemeral;
use crate::fmt::discord::{DisplayCommand, interaction_location};
use crate::helper::futures::noop_future;
use crate::prelude::*;

pub mod args;

use args::SlashMember;

pub mod prelude {
    pub use bson_model::ModelDocument as _;
    pub use houston_cmd::{Context, chat_command, context_command, sub_command};

    pub use super::args::*;
    pub use super::{ContextExt as _, SlashUserOptionExt as _, create_reply};
    pub use crate::helper::discord::components::*;
    pub use crate::prelude::*;
}

/// Pre-command execution hook.
pub fn pre_command(ctx: Context<'_>) -> BoxFuture<'_, ()> {
    log::info!(
        "{}, {}: {}",
        interaction_location(ctx.guild_id(), ctx.interaction.channel.as_ref()),
        ctx.user().name,
        DisplayCommand::new(&ctx.interaction.data, ctx.options()),
    );

    noop_future()
}

/// Command execution error handler.
pub fn error_handler(error: houston_cmd::Error<'_>) -> BoxFuture<'_, ()> {
    return match error {
        houston_cmd::Error::Command { error, ctx } => command_error(ctx, error),
        houston_cmd::Error::ArgInvalid { message, ctx } => {
            let msg = format!("Argument invalid: {message}");
            Box::pin(context_error(ctx, msg.into()))
        },
        houston_cmd::Error::ArgParse { error, input, ctx } => {
            let msg = format!("Argument invalid: {error}\nCaused by input: '{input}'");
            Box::pin(context_error(ctx, msg.into()))
        },
        _ => {
            log::error!("Oh noes, we got an error: {error:?}");
            noop_future()
        },
    };

    fn command_error(ctx: Context<'_>, err: anyhow::Error) -> BoxFuture<'_, ()> {
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

        Box::pin(context_error(ctx, message))
    }

    async fn context_error(ctx: Context<'_>, feedback: Cow<'_, str>) {
        let embed = CreateEmbed::new()
            .description(feedback)
            .color(ERROR_EMBED_COLOR);

        let reply = create_reply(Ephemeral).embed(embed);
        if let Err(err) = ctx.send(reply).await {
            log::error!("Error in error handler: {err:?}");
        }
    }
}

pub fn create_reply<'new>(ephemeral: impl IntoEphemeral) -> CreateReply<'new> {
    CreateReply::new().ephemeral(ephemeral.into_ephemeral())
}

/// Extension trait for the poise context.
pub trait ContextExt<'a> {
    /// Defers the response with the provided ephemerality.
    async fn defer_as(self, ephemeral: impl IntoEphemeral) -> Result;

    /// Gets the ref to the [`HBotData`] in the context.
    #[must_use]
    fn data_ref(self) -> &'a HBotData;

    /// Gets the guild ID of the guild the command was invoked in or a
    /// descriptive error if not in a guild.
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

/// Extension traits for [`Option<_>`] of slash users etc.
pub trait SlashUserOptionExt<'a>: Sized {
    /// The [`Option`] element type.
    type Inner;

    /// If [`Some`], returns [`Ok(value)`][Ok]. Otherwise attempts to extract
    /// the invoking user from `ctx`.
    fn or_invoking(self, ctx: Context<'a>) -> Result<Self::Inner>;
}

impl<'a> SlashUserOptionExt<'a> for Option<SlashMember<'a>> {
    type Inner = SlashMember<'a>;

    fn or_invoking(self, ctx: Context<'a>) -> Result<Self::Inner> {
        match self {
            Some(member) => Ok(member),
            None => SlashMember::from_ctx(ctx),
        }
    }
}
