use std::borrow::Cow;

use serenity::builder::*;
use serenity::model::prelude::*;

use crate::BoxFuture;
use crate::context::Context;
use crate::error::Error;

/// Function to execute for a [`Invoke::ChatInput`] command.
///
/// It is expected that the function extracts its parameters from the
/// [`Context::options`] and validates them itself.
pub type ChatInputFn = for<'i> fn(Context<'i>) -> BoxFuture<'i, Result<(), Error<'i>>>;

/// Function to execute for a [`Invoke::User`] context command.
pub type UserFn = for<'i> fn(
    Context<'i>,
    &'i User,
    Option<&'i PartialMember>,
) -> BoxFuture<'i, Result<(), Error<'i>>>;

/// Function to execute for a [`Invoke::Message`] context command.
pub type MessageFn = for<'i> fn(Context<'i>, &'i Message) -> BoxFuture<'i, Result<(), Error<'i>>>;

/// Function to return autocompletion choices for a command parameter.
///
/// This does not return a [`Result`] as there is no way to communicate that
/// failure to the user.
pub type AutocompleteFn =
    for<'i> fn(Context<'i>, &'i str) -> BoxFuture<'i, CreateAutocompleteResponse<'i>>;

/// Represents a top-level command, as understood by Discord.
///
/// This holds information only relevant to the "root" of a command and the
/// immediate first node.
#[derive(Debug, Clone)]
pub struct Command {
    pub contexts: Option<Cow<'static, [InteractionContext]>>,
    pub integration_types: Option<Cow<'static, [InstallationContext]>>,
    pub default_member_permissions: Option<Permissions>,
    pub nsfw: bool,
    pub data: CommandOption,
}

/// Contains the data for a command or command-group.
#[derive(Debug, Clone)]
pub struct CommandOption {
    pub name: Cow<'static, str>,
    pub description: Cow<'static, str>,
    pub data: CommandOptionData,
}

/// This represents a command option, that is either a command group or an
/// invokable command.
#[derive(Debug, Clone)]
pub enum CommandOptionData {
    Group(GroupData),
    Command(SubCommandData),
}

/// A group of commands.
#[derive(Debug, Clone)]
pub struct GroupData {
    pub sub_commands: Cow<'static, [CommandOption]>,
}

/// A sub-command that may be nested in a group.
/// Also used to represent the invokable information about a root command.
#[derive(Debug, Clone)]
pub struct SubCommandData {
    pub invoke: Invoke,
    pub parameters: Cow<'static, [Parameter]>,
}

/// How the command can be invoked.
#[derive(Debug, Clone, Copy)]
pub enum Invoke {
    ChatInput(ChatInputFn),
    User(UserFn),
    Message(MessageFn),
}

/// A command parameter.
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: Cow<'static, str>,
    pub description: Cow<'static, str>,
    pub required: bool,
    pub autocomplete: Option<AutocompleteFn>,
    pub choices: fn() -> Cow<'static, [Choice]>,
    pub type_setter: fn(CreateCommandOption<'_>) -> CreateCommandOption<'_>,
}

/// A choice value for a command parameter.
#[derive(Debug, Clone)]
pub struct Choice {
    pub name: Cow<'static, str>,
}

impl From<GroupData> for CommandOptionData {
    fn from(value: GroupData) -> Self {
        Self::Group(value)
    }
}

impl From<SubCommandData> for CommandOptionData {
    fn from(value: SubCommandData) -> Self {
        Self::Command(value)
    }
}

impl Command {
    /// Builds a [`CreateCommand`] instance from this value.
    ///
    /// Also see [`crate::to_create_command`] which allows bulk-converting them.
    pub fn to_create_command(&self) -> CreateCommand<'static> {
        let mut command = CreateCommand::new(self.data.name.clone()).nsfw(self.nsfw);

        if let Some(contexts) = &self.contexts {
            command = command.contexts(contexts.to_vec());
        }

        if let Some(integration_types) = &self.integration_types {
            command = command.integration_types(integration_types.to_vec());
        }

        if let Some(permissions) = self.default_member_permissions {
            command = command.default_member_permissions(permissions);
        }

        match &self.data.data {
            CommandOptionData::Group(group) => {
                command = command.description(self.data.description.clone());
                for sub_command in group.sub_commands.iter() {
                    command = command.add_option(sub_command.to_create_command_option());
                }
            },
            CommandOptionData::Command(cmd) => {
                command = match cmd.invoke {
                    Invoke::ChatInput(_) => command
                        .kind(CommandType::ChatInput)
                        .description(self.data.description.clone()),
                    Invoke::User(_) => command.kind(CommandType::User),
                    Invoke::Message(_) => command.kind(CommandType::Message),
                };

                for param in cmd.parameters.iter() {
                    command = command.add_option(param.to_create_command_option());
                }
            },
        }

        command
    }
}

impl CommandOption {
    /// Builds a [`CreateCommandOption`] instance from this value.
    fn to_create_command_option(&self) -> CreateCommandOption<'static> {
        let mut command = CreateCommandOption::new(
            CommandOptionType::SubCommandGroup,
            self.name.clone(),
            self.description.clone(),
        );

        match &self.data {
            CommandOptionData::Group(group) => {
                command = command.kind(CommandOptionType::SubCommandGroup);
                for sub_command in group.sub_commands.iter() {
                    command = command.add_sub_option(sub_command.to_create_command_option());
                }
            },
            CommandOptionData::Command(cmd) => {
                command = command.kind(CommandOptionType::SubCommand);
                for param in cmd.parameters.iter() {
                    command = command.add_sub_option(param.to_create_command_option());
                }
            },
        }

        command
    }
}

impl Parameter {
    /// Builds a [`CreateCommandOption`] instance from this value.
    pub fn to_create_command_option(&self) -> CreateCommandOption<'static> {
        let mut option = CreateCommandOption::new(
            CommandOptionType::String,
            self.name.clone(),
            self.description.clone(),
        )
        .required(self.required)
        .set_autocomplete(self.autocomplete.is_some());

        #[allow(clippy::cast_possible_wrap)]
        for (index, choice) in (self.choices)().iter().enumerate() {
            option = option.add_int_choice(choice.name.clone(), index as i64);
        }

        (self.type_setter)(option)
    }
}
