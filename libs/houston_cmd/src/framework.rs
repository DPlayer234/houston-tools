use std::sync::atomic::{AtomicBool, Ordering};

use serenity::builder::CreateInteractionResponse;
use serenity::futures::future::always_ready;
use serenity::gateway::client::{Context as SerenityContext, FullEvent};
use serenity::model::prelude::*;

use crate::BoxFuture;
use crate::args::{CommandOptionResolver, ResolvedOption};
use crate::context::{Context, ContextInner};
use crate::error::Error;
use crate::model::{Command, CommandOptionData, Invoke, SubCommandData};

type PreCommandFn = fn(Context<'_>) -> BoxFuture<'_, ()>;
type OnErrorFn = fn(Error<'_>) -> BoxFuture<'_, ()>;

/// The command framework itself.
///
/// Can be registered to [serenity's client].
///
/// [serenity's client]: serenity::gateway::client::ClientBuilder::framework
#[derive(Debug, Default)]
pub struct Framework {
    commands: command_set::CommandSet,
    pre_command: Option<PreCommandFn>,
    on_error: Option<OnErrorFn>,
    auto_register: AtomicBool,
}

// expanded `#[async_trait]` impl so the different branch futures can be boxed
// independently. this avoids always allocating the size of the biggest future.
impl serenity::framework::Framework for Framework {
    fn dispatch<'s: 'f, 'c: 'f, 'e: 'f, 'f>(
        &'s self,
        ctx: &'c SerenityContext,
        event: &'e FullEvent,
    ) -> BoxFuture<'f, ()> {
        match event {
            FullEvent::Ready { .. } => Box::pin(self.register_commands(ctx)),
            FullEvent::InteractionCreate {
                interaction: Interaction::Command(interaction),
                ..
            } => Box::pin(self.run_command(ctx, interaction)),
            FullEvent::InteractionCreate {
                interaction: Interaction::Autocomplete(interaction),
                ..
            } => Box::pin(self.run_autocomplete(ctx, interaction)),
            _ => Box::pin(always_ready(|| {})),
        }
    }
}

impl Framework {
    /// Constructs a new empty framework.
    ///
    /// At minimum, you should call [`Self::commands`] to register the supported
    /// commands.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the list of commands.
    ///
    /// Repeated calls replace the entire list.
    #[must_use]
    pub fn commands<I>(mut self, commands: I) -> Self
    where
        I: IntoIterator<Item = Command>,
    {
        self.commands = commands.into_iter().collect();
        self
    }

    /// Sets a function to call before every command invocation.
    #[must_use]
    pub fn pre_command(mut self, pre_command: PreCommandFn) -> Self {
        self.pre_command = Some(pre_command);
        self
    }

    /// Sets the error handler function.
    #[must_use]
    pub fn on_error(mut self, on_error: OnErrorFn) -> Self {
        self.on_error = Some(on_error);
        self
    }

    /// Sets the framework to automatically register all commands globally.
    ///
    /// If there is a need to register commands to a guild specifically, or to
    /// register various commands differently, you will need to handle that
    /// manually. You can use [`Command::to_create_command`] and
    /// [`crate::to_create_command`] to get the appropriate application command
    /// entities.
    #[must_use]
    pub fn auto_register(mut self) -> Self {
        *self.auto_register.get_mut() = true;
        self
    }

    async fn handle_error(&self, why: Error<'_>) {
        match self.on_error {
            Some(on_error) => on_error(why).await,
            None => log::error!("Unhandled command error: {why}"),
        }
    }

    async fn register_commands(&self, ctx: &SerenityContext) {
        // if this was already false, either `auto_register` was not used or we are
        // already registering commands
        if !self.auto_register.swap(false, Ordering::AcqRel) {
            return;
        }

        if let Err(why) = self.register_commands_or(ctx).await {
            // on failure, reset it to true so we might be able to retry later
            self.auto_register.store(true, Ordering::Release);
            log::error!("Failed to register commands: {why}");
        }
    }

    async fn register_commands_or(&self, ctx: &SerenityContext) -> Result<(), serenity::Error> {
        let commands = crate::to_create_command(self.commands.iter());
        let commands = ctx.http.create_global_commands(&commands).await?;

        log::info!("Created {} global commands.", commands.len());
        Ok(())
    }

    async fn run_command(&self, ctx: &SerenityContext, interaction: &CommandInteraction) {
        let (command, options) = match self.find_command(interaction) {
            Ok(r) => r,
            Err(why) => {
                let ctx_inner = ContextInner::empty();
                let ctx = Context::new(ctx, interaction, &ctx_inner);
                self.handle_error(Error::structure_mismatch(ctx, why)).await;
                return;
            },
        };

        let ctx_inner = ContextInner::with_options(options);
        let ctx = Context::new(ctx, interaction, &ctx_inner);
        if let Err(why) = self.run_command_or(ctx, command).await {
            self.handle_error(why).await;
        }
    }

    async fn run_autocomplete(&self, ctx: &SerenityContext, interaction: &CommandInteraction) {
        let (command, options) = match self.find_command(interaction) {
            Ok(r) => r,
            Err(why) => {
                let ctx_inner = ContextInner::empty();
                let ctx = Context::new(ctx, interaction, &ctx_inner);
                self.handle_error(Error::structure_mismatch(ctx, why)).await;
                return;
            },
        };

        let ctx_inner = ContextInner::with_options(options);
        let ctx = Context::new(ctx, interaction, &ctx_inner);
        if let Err(why) = self.run_autocomplete_or(ctx, command).await {
            self.handle_error(why).await;
        }
    }

    async fn run_command_or<'ctx>(
        &self,
        ctx: Context<'ctx>,
        command: &SubCommandData,
    ) -> Result<(), Error<'ctx>> {
        if let Some(pre_command) = self.pre_command {
            pre_command(ctx).await;
        }

        match (ctx.interaction.data.kind, command.invoke) {
            (CommandType::ChatInput, Invoke::ChatInput(invoke)) => invoke(ctx).await,
            (CommandType::User, Invoke::User(invoke)) => {
                let Some(ResolvedTarget::User(user, member)) = ctx.interaction.data.target() else {
                    return Err(Error::structure_mismatch(ctx, "missing user target"));
                };

                invoke(ctx, user, member).await
            },
            (CommandType::Message, Invoke::Message(invoke)) => {
                let Some(ResolvedTarget::Message(message)) = ctx.interaction.data.target() else {
                    return Err(Error::structure_mismatch(ctx, "missing message target"));
                };

                invoke(ctx, message).await
            },
            _ => Err(invoke_structure_mismatch(ctx, command.invoke)),
        }
    }

    async fn run_autocomplete_or<'ctx>(
        &self,
        ctx: Context<'ctx>,
        command: &SubCommandData,
    ) -> Result<(), Error<'ctx>> {
        let Some((name, value)) = ctx.options().iter().find_map(|o| match o.value {
            ResolvedValue::Autocomplete { value, .. } => Some((o.name, value)),
            _ => None,
        }) else {
            return Ok(());
        };

        let Some(parameter) = command.parameters.iter().find(|p| p.name == name) else {
            return Err(Error::structure_mismatch(
                ctx,
                "unknown command autocomplete parameter",
            ));
        };

        let Some(autocomplete) = parameter.autocomplete else {
            return Err(Error::structure_mismatch(
                ctx,
                "expected autocompletable parameter",
            ));
        };

        let interaction = ctx.interaction;
        let http = ctx.http();

        let autocomplete = autocomplete(ctx, value).await;
        let reply = CreateInteractionResponse::Autocomplete(autocomplete);
        if let Err(why) = interaction.create_response(http, reply).await {
            log::warn!("Autocomplete failed: {why:?}");
        }

        Ok(())
    }

    fn find_command<'ctx>(
        &self,
        interaction: &'ctx CommandInteraction,
    ) -> Result<(&SubCommandData, Box<[ResolvedOption<'ctx>]>), &'static str> {
        let data = &interaction.data;
        let name = data.name.as_str();

        // find the root command
        let root = self.commands.get(name).ok_or("unknown command")?;
        let mut resolver = CommandOptionResolver::new(data);

        // traverse the command tree to find the correct sub command
        let mut command = &root.data;
        while let Some(name) = resolver.sub_command() {
            let CommandOptionData::Group(group) = &command.data else {
                return Err("got sub-command when arguments were expected");
            };

            let Some(next_command) = group.sub_commands.iter().find(|c| *c.name == *name) else {
                return Err("unknown sub-command");
            };

            command = next_command;
        }

        let CommandOptionData::Command(command) = &command.data else {
            return Err("got arguments when sub-command was expected");
        };

        let options = resolver.options()?;
        Ok((command, options))
    }
}

#[cold]
fn invoke_structure_mismatch(ctx: Context<'_>, invoke: Invoke) -> Error<'_> {
    let msg = match invoke {
        Invoke::ChatInput(_) => "expected chat input command",
        Invoke::User(_) => "expected user context command",
        Invoke::Message(_) => "expected message context command",
    };
    Error::structure_mismatch(ctx, msg)
}

/// Provides a set/map for commands that avoids having to clone command names to
/// be used as the key of the map.
mod command_set {
    use std::borrow::Borrow;
    use std::collections::HashSet;
    use std::hash::{Hash, Hasher};

    use crate::model::Command;

    /// Internal wrapper around [`Command`] to implement set equality.
    #[derive(Debug)]
    #[repr(transparent)]
    struct Item(Command);

    impl Item {
        fn key(&self) -> &str {
            &self.0.data.name
        }

        fn inner(&self) -> &Command {
            &self.0
        }
    }

    impl PartialEq for Item {
        fn eq(&self, other: &Self) -> bool {
            self.key() == other.key()
        }
    }

    impl Eq for Item {}

    impl Borrow<str> for Item {
        fn borrow(&self) -> &str {
            self.key()
        }
    }

    impl Hash for Item {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.key().hash(state);
        }
    }

    /// Semi-storage-specialized `HashMap<str, Command>`.
    #[derive(Debug, Default)]
    pub struct CommandSet(HashSet<Item>);

    impl CommandSet {
        pub fn get(&self, key: &str) -> Option<&Command> {
            self.0.get(key).map(Item::inner)
        }

        pub fn iter(&self) -> impl Iterator<Item = &Command> {
            self.0.iter().map(Item::inner)
        }
    }

    impl FromIterator<Command> for CommandSet {
        fn from_iter<T: IntoIterator<Item = Command>>(iter: T) -> Self {
            Self(iter.into_iter().map(Item).collect())
        }
    }
}
