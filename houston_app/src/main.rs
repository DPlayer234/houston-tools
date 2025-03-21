mod build;
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
    use std::panic;
    use std::sync::Arc;

    use houston_cmd::Framework;
    use modules::Module as _;
    use serenity::gateway::ActivityData;
    use serenity::prelude::*;

    use crate::build::{GIT_HASH, VERSION};
    use crate::data::cache::Cache;
    use crate::prelude::*;

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

        let bot_data = Arc::new(HBotData::new(config.bot));
        let init = bot_data.init()?;

        let event_handler = HEventHandler::new(bot_data.config());
        let framework = Framework::new()
            .commands(init.commands)
            .pre_command(|ctx| Box::pin(slashies::pre_command(ctx)))
            .on_error(|err| Box::pin(slashies::error_handler(err)))
            .auto_register();

        // note: if any module ever needs access to `Http` at this point, manually
        // create one and use `ClientBuilder::new_with_http` instead.
        // or add a `ready` method to the modules and call that in the event.
        let startup = Arc::clone(&bot_data).startup();
        let discord = async move {
            let mut client = Client::builder(config.discord.token, init.intents)
                .activity(ActivityData::custom(
                    // this accepts `Into<String>`, not `Into<Cow<'_, str>>`
                    config.discord.status.unwrap_or_else(|| VERSION.to_owned()),
                ))
                .raw_event_handler::<Cache>(Arc::clone(&bot_data.cache))
                .framework(framework)
                .event_handler(event_handler)
                .data(Arc::clone(&bot_data))
                .await
                .context("failed to init discord client")?;

            client
                .start()
                .await
                .context("discord client shut down unexpectedly")
        };

        tokio::try_join!(discord, startup)?;
        Ok(())
    }

    /// Custom panic handler that writes the panic to the logger and flushes it.
    ///
    /// This _could_ be a problem if the logger is the cause of the panic, but
    /// at that stage error reporting is already screwed so this doesn't make it
    /// any worse.
    fn on_panic(info: &panic::PanicHookInfo<'_>) {
        use std::backtrace::Backtrace;
        use std::io::{Write as _, stdout};

        // always include the backtrace here, even when not enabled
        // we do this because:
        // - the user already opted into a custom panic handler
        // - not having backtrace on panic is garbage for debugging
        let backtrace = Backtrace::force_capture();
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("<unnamed>");

        // just in case the loggers fail or are empty
        // we could capture and use the default panic handler,
        // but then we might lose out on the backtrace
        _ = writeln!(stdout(), "thread '{name}' {info}");
        log::error!("thread '{name}' {info}\n{backtrace}");
        log::logger().flush();
    }

    /// Type to handle various Discord events.
    struct HEventHandler {
        handlers: Box<[Box<dyn EventHandler>]>,
    }

    impl HEventHandler {
        fn new(config: &config::HBotConfig) -> Self {
            let mut handlers = <Vec<Box<dyn EventHandler>>>::new();

            // fixed handlers
            handlers.push(Box::new(buttons::EventHandler));

            // add module handlers
            handlers.extend(modules::iter_modules!(config, |m| m.event_handler()).flatten());

            Self {
                handlers: handlers.into_boxed_slice(),
            }
        }
    }

    #[serenity::async_trait]
    impl EventHandler for HEventHandler {
        async fn dispatch(&self, ctx: &Context, event: &FullEvent) {
            use serenity::futures::future::join_all;

            // this isn't _super_ optimal since it will allocate a boxed slice of futures,
            // but it's probably a minor thing in the grand scheme. it should also be able
            // to figure out the correct size immediately, so no _redundant_ allocs.
            join_all(self.handlers.iter().map(|h| h.dispatch(ctx, event))).await;
        }
    }

    fn profile() -> Result<Cow<'static, str>> {
        use std::env::VarError::NotPresent;
        use std::env::var;

        match var("HOUSTON_PROFILE") {
            Ok(value) => Ok(value.into()),
            Err(NotPresent) => Ok("release".into()),
            Err(err) => Err(err).context("cannot load HOUSTON_PROFILE env variable"),
        }
    }

    fn build_config() -> Result<config::HConfig> {
        use config::setup::{Builder, Env, File, TomlText};

        let profile = profile()?;
        let profile_config = format!("houston_app.{profile}.toml");
        let default_config = include_str!("../assets/default_config.toml");

        Builder::new()
            .add_layer(TomlText::new(default_config))
            .add_layer(File::new("houston_app.toml").required(false))
            .add_layer(File::new(&profile_config).required(false))
            .add_layer(Env::new())
            .build()
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
