use std::sync::Arc;
use crate::prelude::*;
use utils::discord_fmt::DisplayResolvedArgs;

mod azur;
mod coin;
mod config;
mod dice;
mod timestamp;
mod who;

/// Gets all poise commands.
pub fn get_commands() -> Vec<poise::Command<Arc<HBotData>, HError>> {
    vec![
        azur::azur(),
        coin::coin(),
        config::config(),
        dice::dice(),
        timestamp::timestamp(),
        who::who(),
    ]
}

/// Pre-command execution hook.
pub async fn pre_command(ctx: HContext<'_>) {
    println!("{}: /{} {}", &ctx.author().name, &ctx.command().qualified_name, match ctx {
        HContext::Application(ctx) => {
            ctx.interaction.data.target()
                .map(DisplayResolvedArgs::from_target)
                .unwrap_or_else(|| DisplayResolvedArgs::from_options(ctx.args))
        },
        HContext::Prefix(ctx) => {
            DisplayResolvedArgs::from_str(ctx.args)
        }
    })
}

/// Command execution error handler.
pub async fn error_handler(error: poise::FrameworkError<'_, Arc<HBotData>, HError>) {
    match &error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            context_error(ctx, format_error(error)).await
        },
        poise::FrameworkError::ArgumentParse { error, input, ctx, .. } => {
            context_error(ctx, format!("Argument invalid: {}\nCaused by input: '{}'", error, input.as_deref().unwrap_or_default())).await
        },
        _ => println!("Oh noes, we got an error: {error:?}"),
    }

    async fn context_error(ctx: &HContext<'_>, feedback: String) {
        match ctx.send(ctx.create_ephemeral_reply().embed(CreateEmbed::new().description(feedback).color(ERROR_EMBED_COLOR))).await {
            Err(err) => println!("Error in error handler: {err:?}"),
            _ => () // All good here!
        };
    }

    fn format_error(err: &HError) -> String {
        format!("{err}")
    }
}
