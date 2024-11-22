use std::borrow::Cow;

use derivative::Derivative;
use serenity::builder::{CreateAutocompleteResponse, CreateCommand, CreateCommandOption};
use serenity::model::application::{CommandOptionType, CommandType, InstallationContext, InteractionContext};
use serenity::model::channel::Message;
use serenity::model::guild::PartialMember;
use serenity::model::permissions::Permissions;
use serenity::model::user::User;

use crate::context::Context;
use crate::error::Error;
use crate::BoxFuture;

pub type ChatInputFn = for<'i> fn(Context<'i>) -> BoxFuture<'i, Result<(), Error<'i>>>;
pub type UserFn = for<'i> fn(Context<'i>, &'i User, Option<&'i PartialMember>) -> BoxFuture<'i, Result<(), Error<'i>>>;
pub type MessageFn = for<'i> fn(Context<'i>, &'i Message) -> BoxFuture<'i, Result<(), Error<'i>>>;
pub type AutocompleteFn = for<'i> fn(Context<'i>, &'i str) -> BoxFuture<'i, CreateAutocompleteResponse<'i>>;

/// Represents a top-level command, as understood by Discord.
///
/// This holds information only relevant to the "root" of a command and the immediate first node.
#[derive(Derivative)]
#[derivative(Debug(bound=""), Clone(bound=""))]
pub struct Command {
	pub contexts: Option<Cow<'static, [InteractionContext]>>,
	pub integration_types: Option<Cow<'static, [InstallationContext]>>,
    pub default_member_permissions: Option<Permissions>,
    pub nsfw: bool,
    pub data: CommandOption,
}

#[derive(Derivative)]
#[derivative(Debug(bound=""), Clone(bound=""))]
pub struct CommandOption {
	pub name: Cow<'static, str>,
	pub description: Cow<'static, str>,
    pub data: CommandOptionData,
}

/// This represents a command option, that is either a command group or an invokable command.
#[derive(Derivative)]
#[derivative(Debug(bound=""), Clone(bound=""))]
pub enum CommandOptionData {
	Group(GroupData),
	Command(SubCommandData),
}

/// A group of commands.
#[derive(Derivative)]
#[derivative(Debug(bound=""), Clone(bound=""))]
pub struct GroupData {
	pub sub_commands: Cow<'static, [CommandOption]>,
}

/// A sub-command that may be nested in a group.
/// Also used to represent the invokable information about a root command.
#[derive(Derivative)]
#[derivative(Debug(bound=""), Clone(bound=""))]
pub struct SubCommandData {
	pub invoke: Invoke,
	pub parameters: Cow<'static, [Parameter]>,
}

/// How the command can be invoked.
#[derive(Derivative)]
#[derivative(Debug(bound=""), Clone(bound=""), Copy(bound=""))]
pub enum Invoke {
    ChatInput(ChatInputFn),
    User(UserFn),
    Message(MessageFn),
}

/// A command parameter.
#[derive(Derivative)]
#[derivative(Debug(bound=""), Clone(bound=""))]
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
    pub fn to_application_command(&self) -> CreateCommand<'static> {
        let mut command = CreateCommand::new(self.data.name.clone())
            .nsfw(self.nsfw);

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
                    command = command.add_option(sub_command.to_application_option());
                }
            },
            CommandOptionData::Command(cmd) => {
                command = match cmd.invoke {
                    Invoke::ChatInput(_) => command.kind(CommandType::ChatInput).description(self.data.description.clone()),
                    Invoke::User(_) => command.kind(CommandType::User),
                    Invoke::Message(_) => command.kind(CommandType::Message),
                };

                for param in cmd.parameters.iter() {
                    command = command.add_option(param.to_application_option());
                }
            },
        }

        command
    }
}

impl CommandOption {
    fn to_application_option(&self) -> CreateCommandOption<'static> {
        let mut command = CreateCommandOption::new(CommandOptionType::SubCommandGroup, self.name.clone(), self.description.clone());

        match &self.data {
            CommandOptionData::Group(group) => {
                command = command.kind(CommandOptionType::SubCommandGroup);
                for sub_command in group.sub_commands.iter() {
                    command = command.add_sub_option(sub_command.to_application_option());
                }
            },
            CommandOptionData::Command(cmd) => {
                command = command.kind(CommandOptionType::SubCommand);
                for param in cmd.parameters.iter() {
                    command = command.add_sub_option(param.to_application_option());
                }
            },
        }

        command
    }
}

impl Parameter {
    pub fn to_application_option(&self) -> CreateCommandOption<'static> {
        let mut option = CreateCommandOption::new(CommandOptionType::String, self.name.clone(), self.description.clone())
            .required(self.required)
            .set_autocomplete(self.autocomplete.is_some());

        #[allow(clippy::cast_possible_wrap)]
        for (index, choice) in (self.choices)().iter().enumerate() {
            option = option.add_int_choice(choice.name.clone(), index as i64);
        }

        (self.type_setter)(option)
    }
}
