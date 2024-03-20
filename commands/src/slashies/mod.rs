use crate::internal::prelude::*;
mod config;
mod timestamp;
mod who;

pub fn get_commands() -> Vec<poise::Command<HBotData, HError>> {
    vec![
        config::config(),
        timestamp::timestamp(),
        who::who(),
    ]
}

pub async fn error_handler(error: poise::FrameworkError<'_, HBotData, HError>) {
    match &error {
        poise::FrameworkError::Command { error, ctx, .. } => context_error(ctx, format_error(error)).await,
        poise::FrameworkError::ArgumentParse { error, input, ctx, .. } => context_error(ctx, format!("Argument error: {error:?}\nCaused by input: '{input:?}'")).await,
        _ => println!("Oh noes, we got an error: {:?}", error),
    }

    async fn context_error(ctx: &HContext<'_>, feedback: String) {
        let _ = ctx.send(ctx.create_ephemeral_reply().embed(CreateEmbed::default().description(feedback).color(ERROR_EMBED_COLOR))).await;
    }

    fn format_error(err: &HError) -> String {
        format!("{err}")
    }
}