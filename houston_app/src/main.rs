mod buttons;
mod config;
mod data;
mod fmt;
mod helper;
mod logging;
mod modules;
mod prelude;
mod slashies;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use std::num::NonZero;
    use std::panic;
    use std::sync::{Arc, Mutex};

    use houston_cmd::Framework;
    use serenity::gateway::ActivityData;
    use serenity::prelude::*;

    use crate::prelude::*;

    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const GIT_HASH: &str = match option_env!("GIT_HASH") {
        Some(git_hash) => git_hash,
        None => "<unknown>",
    };

    // run the program and clean up
    let res = run().await;
    if let Err(why) = &res {
        log::error!("Exiting due to error: {why:?}");
    }

    log::logger().flush();
    return res;

    // actual main logic
    async fn run() -> Result {
        // SAFETY: No other code running that accesses this yet.
        unsafe {
            crate::helper::time::mark_startup_time();
        }

        let config = build_config()?;
        init_logging(config.log.log4rs)?;

        if config.log.panic {
            // register the custom panic handler after logging is set up
            panic::set_hook(Box::new(on_panic));
        }

        log::info!(target: "houston_app::version", "Houston Tools v{VERSION} - {GIT_HASH}");

        let mut init = modules::Info::new();
        init.load(&config.bot)?;

        let bot_data = Arc::new(HBotData::new(config.bot));

        bot_data.connect(&init).await?;
        tokio::task::spawn(load_azur_lane(Arc::clone(&bot_data)));

        let event_handler = HEventHandler {
            commands: Mutex::new(Some(houston_cmd::to_create_command(&init.commands))),
        };

        let framework = Framework::new()
            .commands(init.commands)
            .pre_command(|ctx| Box::pin(slashies::pre_command(ctx)))
            .on_error(|err| Box::pin(slashies::error_handler(err)));

        let mut client = Client::builder(config.discord.token, init.intents)
            .activity(ActivityData::custom(
                config
                    .discord
                    .status
                    .map(Cow::Owned)
                    .unwrap_or(Cow::Borrowed(env!("CARGO_PKG_VERSION"))),
            ))
            .data(Arc::clone(&bot_data))
            .framework(framework)
            .event_handler(event_handler)
            .await
            .context("failed to build discord client")?;

        client
            .start()
            .await
            .context("discord client shut down unexpectedly")
    }

    /// Custom panic handler that writes the panic to the logger and flushes it.
    ///
    /// This _could_ be a problem if the logger is the cause of the panic, but
    /// at that stage error reporting is already screwed so this doesn't make it
    /// any worse.
    fn on_panic(info: &panic::PanicHookInfo<'_>) {
        let backtrace = backtrace::Backtrace::new();
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("<unnamed>");

        // just in case the loggers fail or are empty
        eprintln!("thread '{name}' {info}");
        log::error!("thread '{name}' {info}\n{backtrace:?}");
        log::logger().flush();
    }

    /// Type to handle various Discord events.
    struct HEventHandler {
        commands: Mutex<Option<Vec<CreateCommand<'static>>>>,
    }

    #[serenity::async_trait]
    impl EventHandler for HEventHandler {
        async fn ready(&self, ctx: Context, ready: Ready) {
            let discriminator = ready.user.discriminator.map_or(0u16, NonZero::get);
            log::info!("Logged in as: {}#{:04}", ready.user.name, discriminator);

            let data = ctx.data::<HContextData>();
            data.set_current_user(ready.user);

            if let Some(commands) = self.take_commands() {
                if let Err(why) = ready_setup(ctx, &data, &commands).await {
                    log::error!("Failure in ready: {why:?}");
                    *self.commands.lock().unwrap() = Some(commands);
                }
            }
        }

        async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
            modules::perks::dispatch_check_perks(&ctx);
            buttons::handler::interaction_create(ctx, interaction).await;
        }

        async fn message(&self, ctx: Context, new_message: Message) {
            modules::perks::dispatch_check_perks(&ctx);
            modules::media_react::message(ctx, new_message).await;
        }

        async fn message_delete(
            &self,
            ctx: Context,
            channel_id: ChannelId,
            message_id: MessageId,
            guild_id: Option<GuildId>,
        ) {
            modules::starboard::message_delete(ctx, channel_id, message_id, guild_id).await;
        }

        async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
            modules::perks::dispatch_check_perks(&ctx);
            modules::starboard::reaction_add(ctx, reaction).await;
        }
    }

    impl HEventHandler {
        fn take_commands(&self) -> Option<Vec<CreateCommand<'static>>> {
            self.commands.lock().unwrap().take()
        }
    }

    async fn ready_setup(
        ctx: Context,
        data: &HBotData,
        commands: &[CreateCommand<'static>],
    ) -> Result {
        let commands = ctx
            .http()
            .create_global_commands(&commands)
            .await
            .context("failed to create global commands")?;

        log::trace!("Created {} global commands.", commands.len());

        data.load_app_emojis(ctx.http()).await?;
        Ok(())
    }

    async fn load_azur_lane(bot_data: Arc<HBotData>) {
        if bot_data.config().azur_lane_data.is_some() {
            bot_data.force_init();
            log::info!("Loaded Azur Lane data.");
        } else {
            log::trace!("Azur Lane module is disabled.");
        }
    }

    fn profile() -> Result<Cow<'static, str>> {
        use std::env::var;
        use std::env::VarError::NotPresent;

        match var("HOUSTON_PROFILE") {
            Ok(value) => Ok(value.into()),
            Err(NotPresent) => Ok("release".into()),
            Err(err) => Err(err).context("cannot load HOUSTON_PROFILE env variable"),
        }
    }

    fn build_config() -> Result<config::HConfig> {
        use config_rs::{Config, Environment, File, FileFormat};

        let profile = profile()?;
        let profile_config = format!("houston_app.{profile}.toml");

        let config = Config::builder()
            .add_source(File::new("houston_app.toml", FileFormat::Toml).required(false))
            .add_source(File::new(&profile_config, FileFormat::Toml).required(false))
            .add_source(Environment::default().separator("__"))
            // defaults for logging
            .set_default("log.root.level", "warn")?
            .set_default("log.root.appenders[0]", "default")?
            .set_default("log.appenders.default.kind", "default")?
            .set_default("log.appenders.default.encoder.kind", "default")?
            .set_default("log.loggers.houston_app.level", "trace")?
            .build()
            .context("cannot build config")?
            .try_deserialize()
            .context("cannot deserialize config")?;

        Ok(config)
    }

    fn init_logging(config: log4rs::config::RawConfig) -> anyhow::Result<()> {
        let (appenders, errors) = config.appenders_lossy(&logging::deserializers());
        if !errors.is_empty() {
            return Err(errors.into());
        }

        let config = log4rs::Config::builder()
            .appenders(appenders)
            .loggers(config.loggers())
            .build(config.root())?;

        log4rs::init_config(config)?;
        Ok(())
    }
}
