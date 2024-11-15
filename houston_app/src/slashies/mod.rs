use std::borrow::Cow;

use crate::data::IntoEphemeral;
use crate::fmt::discord::DisplayResolvedArgs;
use crate::prelude::*;

/// Pre-command execution hook.
pub async fn pre_command(ctx: HContext<'_>) {
    log::info!("{}: /{} {}", ctx.author().name, ctx.command().qualified_name, match ctx {
        HContext::Application(ctx) => ctx.interaction.data.target().map_or(
            DisplayResolvedArgs::Options(ctx.args),
            DisplayResolvedArgs::Target,
        ),
        HContext::Prefix(ctx) => DisplayResolvedArgs::String(ctx.args),
    })
}

/// Command execution error handler.
#[cold]
pub async fn error_handler(error: poise::FrameworkError<'_, HFrameworkData, HError>) {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            command_error(&ctx, error).await
        },
        poise::FrameworkError::ArgumentParse { error, input, ctx, .. } => {
            context_error(&ctx, format!("Argument invalid: {}\nCaused by input: '{}'", error, input.unwrap_or_default()).into()).await
        },
        _ => log::error!("Oh noes, we got an error: {error:?}"),
    }

    async fn command_error(ctx: &HContext<'_>, err: HError) {
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

    async fn context_error(ctx: &HContext<'_>, feedback: Cow<'_, str>) {
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

macro_rules! command_group {
    ($(#[$meta:meta])* $vis:vis $name:ident $(($($poise_tt:tt)*))? , $($sub_command:literal),* $(,)?) => {
        $(#[$meta])*
        #[::poise::command(
            slash_command,
            subcommands($($sub_command),*),
            subcommand_required,
            $($($poise_tt)*)?
        )]
        $vis async fn $name(_: $crate::data::HContext<'_>) -> $crate::data::HResult {
            $crate::data::HResult::Err($crate::data::HArgError::new_const(concat!(stringify!($name), " cannot be invoked directly")).into())
        }
    };
}

pub(crate) use command_group;
