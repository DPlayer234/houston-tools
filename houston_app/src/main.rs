mod buttons;
mod config;
mod data;
mod modules;
mod fmt;
mod helper;
mod prelude;
mod slashies;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use std::num::NonZero;
    use std::sync::{Arc, Mutex};

    use serenity::model::prelude::*;
    use serenity::prelude::*;

    use data::*;
    use helper::poise_command_builder::CustomCreateCommand;
    use modules::Info;

    // SAFETY: No other code running that accesses this yet.
    unsafe { utils::time::mark_startup_time(); }

    let config = build_config()?;
    init_logging(config.log);

    match option_env!("GIT_HASH") {
        Some(git_hash) => log::info!("Houston Tools [Commit: {git_hash}]"),
        None => log::info!("Houston Tools [Unknown Commit]"),
    };

    log::info!("Starting...");

    let mut init = Info::new();
    init.load(&config.bot)?;

    let bot_data = Arc::new(HBotData::new(config.bot));

    bot_data.connect(&init).await?;
    let loader = tokio::task::spawn(
        load_azur_lane(Arc::clone(&bot_data))
    );

    let event_handler = HEventHandler {
        commands: Mutex::new(Some(helper::poise_command_builder::build_commands(&init.commands))),
    };

    let framework = HFramework::builder()
        .options(poise::FrameworkOptions {
            commands: init.commands,
            pre_command: |ctx| Box::pin(slashies::pre_command(ctx)),
            on_error: |err| Box::pin(slashies::error_handler(err)),
            ..Default::default()
        })
        .build();

    let mut client = Client::builder(&config.discord.token, init.intents)
        .data(Arc::clone(&bot_data))
        .framework(framework)
        .event_handler(event_handler)
        .await?;

    client.start().await?;
    loader.await?;

    return Ok(());

    /// Type to handle various Discord events.
    struct HEventHandler {
        commands: Mutex<Option<Vec<CustomCreateCommand>>>,
    }

    #[serenity::async_trait]
    impl EventHandler for HEventHandler {
        async fn ready(&self, ctx: Context, ready: Ready) {
            let discriminator = ready.user.discriminator.map_or(0u16, NonZero::get);
            log::info!("Logged in as: {}#{:04}", ready.user.name, discriminator);

            if let Some(commands) = self.take_commands() {
                let data = ctx.data::<HFrameworkData>();
                if let Err(why) = Self::setup(ctx, &data, &commands).await {
                    log::error!("Failure in ready: {why:?}");
                    *self.commands.lock().unwrap() = Some(commands);
                }
            }
        }

        async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
            buttons::handler::interaction_create(ctx, interaction).await;
        }

        async fn message(&self, ctx: Context, _new_message: Message) {
            modules::perks::check_perks(ctx).await;
        }

        async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
            modules::starboard::handle_reaction(ctx, reaction).await;
        }
    }

    impl HEventHandler {
        fn take_commands(&self) -> Option<Vec<CustomCreateCommand>> {
            self.commands.lock().unwrap().take()
        }

        async fn setup(
            ctx: Context,
            data: &HBotData,
            commands: &[CustomCreateCommand],
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
}
