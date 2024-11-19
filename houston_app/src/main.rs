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
    use std::borrow::Cow;
    use std::num::NonZero;
    use std::sync::{Arc, Mutex};

    use serenity::builder::CreateCommand;
    use serenity::gateway::ActivityData;
    use serenity::model::prelude::*;
    use serenity::prelude::*;

    use data::*;
    use modules::Info;

    // SAFETY: No other code running that accesses this yet.
    unsafe { crate::helper::time::mark_startup_time(); }

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
        commands: Mutex::new(Some(poise::builtins::create_application_commands(&init.commands))),
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
        .activity(ActivityData::custom(
            config.discord.status
                .map(Cow::Owned)
                .unwrap_or(Cow::Borrowed(env!("CARGO_PKG_VERSION")))
        ))
        .data(Arc::clone(&bot_data))
        .framework(framework)
        .event_handler(event_handler)
        .await?;

    client.start().await?;
    loader.await?;

    return Ok(());

    /// Type to handle various Discord events.
    struct HEventHandler {
        commands: Mutex<Option<Vec<CreateCommand<'static>>>>,
    }

    #[serenity::async_trait]
    impl EventHandler for HEventHandler {
        async fn ready(&self, ctx: Context, ready: Ready) {
            let discriminator = ready.user.discriminator.map_or(0u16, NonZero::get);
            log::info!("Logged in as: {}#{:04}", ready.user.name, discriminator);

            let data = ctx.data::<HFrameworkData>();
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

        async fn message_delete(&self, ctx: Context, channel_id: ChannelId, message_id: MessageId, guild_id: Option<GuildId>) {
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
    ) -> anyhow::Result<()> {
        let commands = ctx.http().create_global_commands(&commands).await?;
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

    fn profile() -> anyhow::Result<Cow<'static, str>> {
        use std::env::{var, VarError::NotPresent};

        match var("HOUSTON_PROFILE") {
            Ok(value) => Ok(value.into()),
            Err(NotPresent) => Ok("release".into()),
            Err(err) => Err(err.into()),
        }
    }

    fn build_config() -> anyhow::Result<config::HConfig> {
        use config_rs::{Config, Environment, File, FileFormat};

        let profile = profile()?;
        let config = Config::builder()
            .add_source(File::new("houston_app.toml", FileFormat::Toml).required(false))
            .add_source(File::new(&format!("houston_app.{profile}.toml"), FileFormat::Toml).required(false))
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
