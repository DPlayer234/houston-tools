use std::fmt::Write;
use crate::internal::prelude::*;
use utils::Discard;

mod coin;
mod config;
mod dice;
mod timestamp;
mod who;

pub fn get_commands() -> Vec<poise::Command<HBotData, HError>> {
    vec![
        coin::coin(),
        config::config(),
        dice::dice(),
        timestamp::timestamp(),
        who::who(),
    ]
}

pub async fn pre_command(ctx: HContext<'_>) {
    println!("{}: /{} {}", &ctx.author().name, &ctx.command().qualified_name, match ctx {
        HContext::Application(ctx) => {
            if let Some(target) = ctx.interaction.data.target() {
                format_resolved_target(&target)
            } else {
                format_resolved_options(ctx.args)
            }
        },
        HContext::Prefix(ctx) => ctx.args.to_owned()
    })
}

pub async fn error_handler(error: poise::FrameworkError<'_, HBotData, HError>) {
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

fn format_resolved_options(options: &[ResolvedOption<'_>]) -> String {
    let mut str = String::new();
    for o in options {
        str.push_str(o.name);
        str.push(':');
        append_resolve_option(&mut str, o);
        str.push(' ');
    }

    str
}

fn append_resolve_option(str: &mut String, option: &ResolvedOption<'_>) {
    match option.value {
        ResolvedValue::Boolean(v) => { write!(str, "{}", v).discard() },
        ResolvedValue::Integer(v) => { write!(str, "{}", v).discard() },
        ResolvedValue::Number(v) => { write!(str, "{}", v).discard() },
        ResolvedValue::String(v) => { str.push('"'); str.push_str(v); str.push('"') },
        ResolvedValue::Attachment(v) => { str.push_str(&v.filename) },
        ResolvedValue::Channel(v) => { if let Some(ref name) = v.name { str.push_str(name) } else { write!(str, "{}", v.id).discard() } },
        ResolvedValue::Role(v) => { str.push_str(&v.name) },
        ResolvedValue::User(v, _) => { str.push_str(&v.name) },
        _ => { str.push_str("<unknown>") },
    }
}

fn format_resolved_target(target: &ResolvedTarget<'_>) -> String {
    match target {
        ResolvedTarget::User(v, _) => v.name.clone(),
        ResolvedTarget::Message(v) => v.id.to_string(),
        _ => "unknown".to_owned(),
    }
}
