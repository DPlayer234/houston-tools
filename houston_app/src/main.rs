use std::num::NonZeroU16;
use std::sync::Arc;

use serenity::model::prelude::*;
use serenity::prelude::*;

mod buttons;
mod slashies;
mod config;
mod data;
mod fmt;
mod prelude;
mod poise_command_builder;

use data::*;

#[tokio::main]
async fn main() {
    let config = build_config();
    let intents = GatewayIntents::empty();

    // SAFETY: No other code running that accesses this yet.
    unsafe { utils::time::mark_startup_time(); }

    println!("Starting...");

    let start = std::time::Instant::now();
    let bot_data = Arc::new(HBotData::new(config.bot));

    let loader = tokio::task::spawn({
        let bot_data = Arc::clone(&bot_data);
        async move {
            if bot_data.config().azur_lane_data.is_some() {
                println!("Loading Azur Lane data...");
                bot_data.force_init();
                println!("Loaded Azur Lane data. ({:.2?})", start.elapsed());
            } else {
                println!("Disabled Azur Lane module.");
            }
        }
    });

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: slashies::get_commands(bot_data.config()),
            pre_command: |ctx| Box::pin(slashies::pre_command(ctx)),
            on_error: |err| Box::pin(slashies::error_handler(err)),
            ..Default::default()
        })
        .setup({
            let bot_data = Arc::clone(&bot_data);
            move |ctx, ready, framework| {
                Box::pin(async move {
                    create_commands(ctx, framework).await?;

                    let discriminator = ready.user.discriminator.map_or(0u16, NonZeroU16::get);
                    println!("Logged in as: {}#{:04} ({:.2?})", ready.user.name, discriminator, start.elapsed());

                    Ok(bot_data)
                })
            }
        })
        .build();

    let mut client = Client::builder(config.discord.token, intents)
        .framework(framework)
        .event_handler(buttons::ButtonEventHandler::new(bot_data))
        .await.unwrap();

    client.start().await.unwrap();
    loader.await.unwrap();
}

async fn create_commands(ctx: &Context, framework: &poise::framework::Framework<Arc<HBotData>, HError>) -> HResult {
    let cmds = poise_command_builder::build_commands(&framework.options().commands);
    let res = ctx.http().create_global_commands(&cmds).await;
    if res.is_err() { println!("{res:?}"); }
    res?;

    Ok(())
}

fn build_config() -> config::HConfig {
    use config_rs::{Config, File, FileFormat, Environment};

    Config::builder()
        .add_source(
            File::new("houston_app.toml", FileFormat::Toml)
                .required(false)
        )
        .add_source(
            Environment::default()
                .separator("__")
        )
        .build().unwrap()
        .try_deserialize().unwrap()
}
