use std::num::NonZero;
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

type HFramework = poise::framework::Framework<Arc<HBotData>, HError>;

const INTENTS: GatewayIntents = GatewayIntents::empty();

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // SAFETY: No other code running that accesses this yet.
    unsafe { utils::time::mark_startup_time(); }

    let config = build_config()?;
    init_logging(config.log);

    log::info!("Starting...");

    let bot_data = Arc::new(HBotData::new(config.bot));

    let loader = tokio::task::spawn(
        load_azur_lane(Arc::clone(&bot_data))
    );

    let framework = HFramework::builder()
        .options(poise::FrameworkOptions {
            commands: slashies::get_commands(bot_data.config()),
            pre_command: |ctx| Box::pin(slashies::pre_command(ctx)),
            on_error: |err| Box::pin(slashies::error_handler(err)),
            ..Default::default()
        })
        .setup({
            let bot_data = Arc::clone(&bot_data);
            move |ctx, ready, framework| Box::pin(async move {
                create_commands(ctx, framework).await?;
                bot_data.load_app_emojis(ctx.http()).await?;

                let discriminator = ready.user.discriminator.map_or(0u16, NonZero::get);
                log::info!("Logged in as: {}#{:04}", ready.user.name, discriminator);

                Ok(bot_data)
            })
        })
        .build();

    let mut client = Client::builder(config.discord.token, INTENTS)
        .framework(framework)
        .event_handler(buttons::ButtonEventHandler::new(bot_data))
        .await?;

    client.start().await?;
    loader.await?;

    Ok(())
}

async fn create_commands(ctx: &Context, framework: &HFramework) -> HResult {
    let cmds = poise_command_builder::build_commands(&framework.options().commands);
    match ctx.http().create_global_commands(&cmds).await {
        Ok(cmds) => {
            log::trace!("Created {} global commands.", cmds.len());
            Ok(())
        }
        Err(err) => {
            log::error!("{err:?}");
            Err(err.into())
        }
    }
}

async fn load_azur_lane(bot_data: Arc<HBotData>) {
    if bot_data.config().azur_lane_data.is_some() {
        bot_data.force_init();
        log::info!("Loaded Azur Lane data.");
    } else {
        log::trace!("Azur Lane module is disabled.");
    }
}

fn build_config() -> anyhow::Result<config::HConfig> {
    use config_rs::{Config, File, FileFormat, Environment};

    let config = Config::builder()
        .add_source(
            File::new("houston_app.toml", FileFormat::Toml)
                .required(false)
        )
        .add_source(
            Environment::default()
                .separator("__")
        )
        .build()?
        .try_deserialize()?;

    Ok(config)
}

fn init_logging(config: config::HLogConfig) {
    use log::LevelFilter;

    let mut builder = env_logger::builder();

    // if no default is specified, set it to warn for everything,
    // but to trace for the main app crate
    match config.default {
        None => builder.filter_level(LevelFilter::Warn).filter_module(std::module_path!(), LevelFilter::Trace),
        Some(value) => builder.filter_level(value),
    };

    for (module, level) in config.modules {
        builder.filter_module(&module, level);
    }

    builder.init();
}
