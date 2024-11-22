use std::mem::take;
use std::sync::atomic::AtomicBool;

use serenity::async_trait;
use serenity::builder::CreateInteractionResponse;
use serenity::framework::Framework as SerenityFramework;
use serenity::gateway::client::{Context as SerenityContext, FullEvent};
use serenity::model::application::{CommandInteraction, CommandType, Interaction, ResolvedOption, ResolvedValue};

use crate::context::Context;
use crate::error::Error;
use crate::model::{Command, CommandOptionData, Invoke, SubCommandData};
use crate::BoxFuture;

type PreCommandFn = fn(Context<'_>) -> BoxFuture<'_, ()>;
type OnErrorFn = fn(Error<'_>) -> BoxFuture<'_, ()>;

pub struct Framework {
    commands: Vec<Command>,
    pre_command: Option<PreCommandFn>,
    on_error: Option<OnErrorFn>,
}

#[async_trait]
impl SerenityFramework for Framework {
    async fn dispatch(&self, ctx: &SerenityContext, event: &FullEvent) {
        match event {
            FullEvent::InteractionCreate {
                interaction: Interaction::Command(interaction)
            } => {
                self.run_command(ctx, interaction).await
            },
            FullEvent::InteractionCreate {
                interaction: Interaction::Autocomplete(interaction)
            } => {
                self.run_autocomplete(ctx, interaction).await
            },
            _ => {},
        }
    }
}

impl Framework {
    #[must_use]
    pub fn builder() -> FrameworkBuilder {
        FrameworkBuilder::default()
    }

    async fn handle_error(&self, why: Error<'_>) {
        match self.on_error {
            Some(on_error) => on_error(why).await,
            None => log::error!("unhandled command error: {why}"),
        }
    }

    async fn run_command(&self, ctx: &SerenityContext, interaction: &CommandInteraction) {
        let reply_state = AtomicBool::new(false);
        let mut ctx = Context::new(&reply_state, ctx, interaction);

        let (command, options) = match self.find_command(interaction) {
            Ok(r) => r,
            Err(why) => {
                self.handle_error(Error::structure_mismatch(ctx, why)).await;
                return;
            }
        };

        ctx.options = &options;
        if let Err(why) = self.run_command_or(ctx, command).await {
            self.handle_error(why).await;
        }
    }

    async fn run_autocomplete(&self, ctx: &SerenityContext, interaction: &CommandInteraction) {
        let reply_state = AtomicBool::new(false);
        let mut ctx = Context::new(&reply_state, ctx, interaction);

        let (command, options) = match self.find_command(interaction) {
            Ok(r) => r,
            Err(why) => {
                self.handle_error(Error::structure_mismatch(ctx, why)).await;
                return;
            }
        };

        ctx.options = &options;
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

        let data = &ctx.interaction.data;
        match data.kind {
            CommandType::ChatInput => {
                let Invoke::ChatInput(invoke) = command.invoke
                else { return Err(Error::structure_mismatch(ctx, "expected chat input command")); };

                invoke(ctx).await
            },
            CommandType::User => {
                let (Invoke::User(invoke), Some(target_id)) = (command.invoke, data.target_id)
                else { return Err(Error::structure_mismatch(ctx, "expected user context command")); };

                let target_id = target_id.to_user_id();
                let Some(user) = data.resolved.users.get(&target_id)
                else { return Err(Error::structure_mismatch(ctx, "expected user target")); };

                let member = data.resolved.members.get(&target_id);
                invoke(ctx, user, member).await
            },
            CommandType::Message => {
                let (Invoke::Message(invoke), Some(target_id)) = (command.invoke, data.target_id)
                else { return Err(Error::structure_mismatch(ctx, "expected message context command")); };

                let target_id = target_id.to_message_id();
                let Some(message) = data.resolved.messages.get(&target_id)
                else { return Err(Error::structure_mismatch(ctx, "expected message target")); };

                invoke(ctx, message).await
            },
            _ => Err(Error::structure_mismatch(ctx, "invalid command type")),
        }
    }

    async fn run_autocomplete_or<'ctx>(
        &self,
        ctx: Context<'ctx>,
        command: &SubCommandData,
    ) -> Result<(), Error<'ctx>> {
        let Some((name, value)) = ctx
            .options()
            .iter()
            .find_map(|o| match o.value {
                ResolvedValue::Autocomplete { value, .. } => Some((o.name, value)),
                _ => None,
            })
        else {
            return Ok(());
        };

        let Some(parameter) = command
            .parameters
            .iter()
            .find(|p| p.name == name)
        else {
            return Err(Error::structure_mismatch(ctx, "unknown command autocomplete parameter"));
        };

        let Some(autocomplete) = parameter.autocomplete
        else { return Err(Error::structure_mismatch(ctx, "expected autocompletable parameter")); };

        let interaction = ctx.interaction;
        let http = ctx.http();

        let autocomplete = autocomplete(ctx, value).await;
        let reply = CreateInteractionResponse::Autocomplete(autocomplete);
        if let Err(why) = interaction.create_response(http, reply).await {
            log::warn!("Autocomplete failed: {why:?}");
        }

        Ok(())
    }

    #[allow(clippy::type_complexity)]
    fn find_command<'ctx>(
        &self,
        interaction: &'ctx CommandInteraction,
    ) -> Result<
        (&SubCommandData, Vec<ResolvedOption<'ctx>>),
        &'static str,
    > {
        let data = &interaction.data;
        let name = data.name.as_str();
        let mut options = data.options();

        // find the root command
        let Some(root) = self.commands
            .iter()
            .find(|c| c.data.name == name)
        else {
            return Err("unknown command");
        };

        // traverse the command tree to find the correct sub command
        let mut command = &root.data;
        while let Some(ResolvedOption {
            name,
            value: ResolvedValue::SubCommand(next_options) | ResolvedValue::SubCommandGroup(next_options),
            ..
        }) = options.first_mut() {
            let CommandOptionData::Group(group) = &command.data
            else {
                return Err("found arguments when command was expected");
            };

            let Some(next_command) = group.sub_commands
                .iter()
                .find(|c| *c.name == **name)
            else {
                return Err("unknown sub-command");
            };

            command = next_command;
            options = take(next_options).into_vec();
        }

        let CommandOptionData::Command(command) = &command.data
        else {
            return Err("found group where command was expected");
        };

        Ok((command, options))
    }
}

pub struct FrameworkBuilder {
    inner: Framework,
}

impl FrameworkBuilder {
    #[must_use]
    pub fn commands(mut self, commands: Vec<Command>) -> Self {
        self.inner.commands = commands;
        self
    }

    #[must_use]
    pub fn pre_command(mut self, pre_command: PreCommandFn) -> Self {
        self.inner.pre_command = Some(pre_command);
        self
    }

    #[must_use]
    pub fn on_error(mut self, on_error: OnErrorFn) -> Self {
        self.inner.on_error = Some(on_error);
        self
    }

    #[must_use]
    pub fn build(self) -> Framework {
        self.inner
    }
}

impl Default for FrameworkBuilder {
    fn default() -> Self {
        Self {
            inner: Framework {
                commands: Vec::new(),
                pre_command: None,
                on_error: None,
            }
        }
    }
}
