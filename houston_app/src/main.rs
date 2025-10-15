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

fn main() -> anyhow::Result<()> {
    use std::panic;

    use houston_cmd::Framework;
    use serenity::gateway::ActivityData;
    use serenity::prelude::*;

    use crate::build::{GIT_HASH, VERSION};
    use crate::config::HConfig;
    use crate::data::cache::CacheUpdateHandler;
    use crate::helper::discord::events::HEventHandler;
    use crate::prelude::*;

    return inner();

    // short async fn to reduce `tokio::main` scope
    #[tokio::main]
    async fn inner() -> anyhow::Result<()> {
        // run the program and clean up
        let res = run().await;
        if let Err(why) = &res {
            log::error!("Exiting due to error: {why:?}");
        }

        log::logger().flush();
        res
    }

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

        let event_handler = HEventHandler::new(init.event_handlers.into_boxed_slice());

        let framework = Framework::new()
            .commands(init.commands)
            .pre_command(slashies::pre_command)
            .on_error(slashies::error_handler)
            .auto_register();

        // note: if any module ever needs access to `Http` at this point, manually
        // create one and use `ClientBuilder::new_with_http` instead.
        let startup = Arc::clone(&bot_data).startup();
        let discord = async move {
            let status = config
                .discord
                .status
                .map(String::from)
                .unwrap_or_else(|| VERSION.to_owned());

            let mut client = Client::builder(config.discord.token, init.intents)
                .activity(ActivityData::custom(status))
                .raw_event_handler(CacheUpdateHandler)
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

    fn profile() -> Result<Cow<'static, str>> {
        use std::env::VarError::NotPresent;
        use std::env::var;

        match var("HOUSTON_PROFILE") {
            Ok(value) => Ok(value.into()),
            Err(NotPresent) => Ok("release".into()),
            Err(err) => Err(err).context("cannot load HOUSTON_PROFILE env variable"),
        }
    }

    fn build_config() -> Result<HConfig> {
        use crate::config::setup::{Builder, Env, File, TomlText};

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
        let deserializers = crate::logging::deserializers();
        let (appenders, errors) = config.appenders_lossy(&{ deserializers });
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
