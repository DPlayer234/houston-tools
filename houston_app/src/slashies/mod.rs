use std::borrow::Cow;
use std::fmt;

use houston_cmd::Context;

use crate::data::IntoEphemeral;
use crate::fmt::discord::DisplayResolvedArgs;
use crate::prelude::*;

pub mod args;

pub mod prelude {
    pub use houston_cmd::{chat_command, context_command};
    pub use houston_cmd::Context;

    pub use super::args::*;
    pub use super::create_reply;
    pub use super::ContextExt as _;
    pub use crate::prelude::*;
}

/// Pre-command execution hook.
pub async fn pre_command(ctx: Context<'_>) {
    let options = ctx
        .interaction
        .data.target()
        .map_or_else(
            || DisplayResolvedArgs::Options(ctx.options()),
            DisplayResolvedArgs::Target,
        );

    struct Tree<'a>(&'a CommandInteraction);
    impl fmt::Display for Tree<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.0.data.name)?;
            let mut options = &self.0.data.options;
            while let Some(CommandDataOption {
                name,
                value: CommandDataOptionValue::SubCommand(next_options) | CommandDataOptionValue::SubCommandGroup(next_options),
                ..
            }) = options.first() {
                f.write_str(" ")?;
                f.write_str(name)?;
                options = next_options;
            }

            Ok(())
        }
    }

    log::info!("{}: /{} {options}", ctx.user().name, Tree(ctx.interaction))
}

/// Command execution error handler.
#[cold]
pub async fn error_handler(error: houston_cmd::Error<'_>) {
    match error {
        houston_cmd::Error::Command { error, ctx } => {
            command_error(ctx, error).await
        },
        houston_cmd::Error::ArgInvalid { message, ctx } => {
            let msg = format!("Argument invalid: {}", message);
            context_error(ctx, msg.into()).await
        },
        houston_cmd::Error::ArgumentParse { error, input, ctx } => {
            let msg = match input {
                Some(input) => format!("Argument invalid: {}\nCaused by input: '{}'", error, input),
                None => format!("Argument invalid: {}", error),
            };

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

                format!("Internal error: ```{err}```")
                    .into()
            }
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
    CreateReply::new()
        .ephemeral(ephemeral.into_ephemeral())
}

/// Extension trait for the poise context.
pub trait ContextExt<'a> {
    async fn defer_as(&self, ephemeral: impl IntoEphemeral) -> Result;

    #[must_use]
    fn data_ref(&self) -> &'a HBotData;

    fn require_guild_id(&self) -> anyhow::Result<GuildId>;
}

impl<'a> ContextExt<'a> for Context<'a> {
    async fn defer_as(&self, ephemeral: impl IntoEphemeral) -> Result {
        self.defer(ephemeral.into_ephemeral()).await?;
        Ok(())
    }

    fn data_ref(&self) -> &'a HBotData {
        self.serenity.data_ref::<HContextData>()
    }

    fn require_guild_id(&self) -> anyhow::Result<GuildId> {
        self.guild_id().context("must be used in guild")
    }
}
