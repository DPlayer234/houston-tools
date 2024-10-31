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

type HFramework = poise::framework::Framework<HBotData, HError>;

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

    let commands = slashies::get_commands(bot_data.config());

    let event_handler = HEventHandler {
        commands: std::sync::Mutex::new(Some(poise_command_builder::build_commands(&commands))),
    };

    let framework = HFramework::builder()
        .options(poise::FrameworkOptions {
            commands,
            pre_command: |ctx| Box::pin(slashies::pre_command(ctx)),
            on_error: |err| Box::pin(slashies::error_handler(err)),
            ..Default::default()
        })
        .build();

    let mut client = Client::builder(&config.discord.token, INTENTS)
        .data(Arc::clone(&bot_data))
        .framework(framework)
        .event_handler(event_handler)
        .await?;

    client.start().await?;
    loader.await?;

    Ok(())
}

struct HEventHandler {
    commands: std::sync::Mutex<Option<Vec<poise_command_builder::CustomCreateCommand>>>,
}

#[serenity::async_trait]
impl EventHandler for HEventHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        let discriminator = ready.user.discriminator.map_or(0u16, NonZero::get);
        log::info!("Logged in as: {}#{:04}", ready.user.name, discriminator);

        let commands = self.commands.lock().unwrap().take();
        if let Some(commands) = commands {
            let data = ctx.data();
            if let Err(why) = Self::setup(ctx, &data, &commands).await {
                log::error!("Failure in ready: {why:?}");
                *self.commands.lock().unwrap() = Some(commands);
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        buttons::handler::interaction_create(ctx, interaction).await
    }
}

impl HEventHandler {
    async fn setup(
        ctx: Context,
        data: &HBotData,
        commands: &[poise_command_builder::CustomCreateCommand],
    ) -> anyhow::Result<()> {
        let commands = ctx.http().create_global_commands(&commands).await?;
        log::trace!("Created {} global commands.", commands.len());

        data.load_app_emojis(ctx.http()).await?;
        Ok(())
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
    use config_rs::{Config, Environment, File, FileFormat};

    let config = Config::builder()
        .add_source(File::new("houston_app.toml", FileFormat::Toml).required(false))
        .add_source(Environment::default().separator("__"))
        .build()?
        .try_deserialize()?;

    Ok(config)
}

fn init_logging(config: config::HLogConfig) {
    use log::LevelFilter;

    let mut builder = env_logger::builder();

    // doing the detection and format ourselves removes the
    // anstream dependency and some other related crates

    // env_logger defaults to using stderr
    let has_color = config.color
        .unwrap_or_else(|| utils::term::supports_ansi_escapes(&std::io::stderr()));

    if has_color {
        builder.format(fmt::log::format_styled);
    } else {
        builder.format(fmt::log::format_unstyled);
    }

    match config.default {
        Some(value) => builder.filter_level(value),

        // if no default is specified, set it to warn for everything,
        // but to trace for the main app crate
        None => builder
            .filter_level(LevelFilter::Warn)
            .filter_module(std::module_path!(), LevelFilter::Trace),
    };

    for (module, level) in config.modules {
        builder.filter_module(&module, level);
    }

    builder.init();
}
