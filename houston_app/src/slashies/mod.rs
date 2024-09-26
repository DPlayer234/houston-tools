use std::sync::Arc;

use crate::fmt::discord::DisplayResolvedArgs;
use crate::prelude::*;

mod azur;
mod calc;
mod coin;
mod config;
mod dice;
mod quote;
mod timestamp;
mod upload;
mod who;

/// Gets all poise commands.
pub fn get_commands(config: &crate::config::HBotConfig) -> Vec<poise::Command<Arc<HBotData>, HError>> {
    let mut result = vec![
        coin::coin(),
        config::config(),
        dice::dice(),
        calc::calc(),
        quote::quote(),
        timestamp::timestamp(),
        who::who(),
        upload::upload(),
    ];

    if config.azur_lane_data.is_some() {
        result.push(azur::azur());
    }

    result
}

/// Pre-command execution hook.
pub async fn pre_command(ctx: HContext<'_>) {
    log::info!("{}: /{} {}", ctx.author().name, ctx.command().qualified_name, match ctx {
        HContext::Application(ctx) => {
            ctx.interaction.data.target()
                .map(DisplayResolvedArgs::Target)
                .unwrap_or_else(|| DisplayResolvedArgs::Options(ctx.args))
        },
        HContext::Prefix(ctx) => {
            DisplayResolvedArgs::String(ctx.args)
        }
    })
}

/// Command execution error handler.
#[cold]
pub async fn error_handler(error: poise::FrameworkError<'_, Arc<HBotData>, HError>) {
    match &error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            command_error(ctx, error).await
        },
        poise::FrameworkError::ArgumentParse { error, input, ctx, .. } => {
            context_error(ctx, format!("Argument invalid: {}\nCaused by input: '{}'", error, input.as_deref().unwrap_or_default())).await
        },
        _ => log::error!("Oh noes, we got an error: {error:?}"),
    }

    async fn command_error(ctx: &HContext<'_>, err: &HError) {
        let message = match err.downcast_ref::<HArgError>() {
            Some(err) => {
                format!("Command error: ```{err}```")
            }
            None => {
                log::error!("Error in command: {err:?}");
                format!("Internal error: ```{err}```")
            }
        };

        context_error(ctx, message).await
    }

    async fn context_error(ctx: &HContext<'_>, feedback: String) {
        let embed = CreateEmbed::new()
            .description(feedback)
            .color(ERROR_EMBED_COLOR);

        let reply = ctx.create_ephemeral_reply().embed(embed);
        if let Err(err) = ctx.send(reply).await {
            log::error!("Error in error handler: {err:?}")
        };
    }
}

macro_rules! command_group {
    ($(#[$meta:meta])* $vis:vis $name:ident, $($sub_command:literal),* $(,)?) => {
        $(#[$meta])*
        #[::poise::command(
            slash_command,
            subcommands($($sub_command),*),
            subcommand_required
        )]
        $vis async fn $name(_: $crate::data::HContext<'_>) -> $crate::data::HResult {
            $crate::data::HResult::Err($crate::data::HArgError(concat!(stringify!($name), " cannot be invoked directly")).into())
        }
    };
}

use command_group;
