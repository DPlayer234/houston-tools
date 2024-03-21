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

pub async fn pre_command(ctx: HContext<'_>) {
    println!("{}: /{} {}", &ctx.author().name, &ctx.command().qualified_name, match ctx {
        // TODO: Context menu commands don't hold their args here
        HContext::Application(ctx) => format_resolved_options(ctx.args),
        HContext::Prefix(ctx) => ctx.args.to_owned()
    })
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

fn format_resolved_options(option: &[ResolvedOption<'_>]) -> String {
    let mut str = String::new();
    for o in option {
        str.push('<');
        str.push_str(o.name);
        str.push(':');

        match o.value {
            ResolvedValue::Boolean(v) => { str.push_str(&v.to_string()) },
            ResolvedValue::Integer(v) => { str.push_str(&v.to_string()) },
            ResolvedValue::Number(v) => { str.push_str(&v.to_string()) },
            ResolvedValue::String(v) => { str.push_str(v) },
            ResolvedValue::Attachment(v) => { str.push_str(&v.filename) },
            ResolvedValue::Channel(v) => { if let Some(ref name) = v.name { str.push_str(name) } else { str.push_str(&v.id.to_string()) } },
            ResolvedValue::Role(v) => { str.push_str(&v.name) },
            ResolvedValue::User(v, _) => { str.push_str(&v.name) },
            _ => { str.push_str("<unknown>") },
        }

        str.push_str("> ");
    }

    str
}