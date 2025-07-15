use std::borrow::Cow;

use const_builder::ConstBuilder;
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

// no PartialEq derives because that would involve comparing function pointers
// and that is far from reliable.

/// Represents a top-level command, as understood by Discord.
///
/// This holds information only relevant to the "root" of a command and the
/// immediate first node.
#[derive(Debug, Clone, ConstBuilder)]
#[non_exhaustive]
pub struct Command {
    /// The interaction contexts the command is available in.
    #[builder(default = "None")]
    pub contexts: Option<Cow<'static, [InteractionContext]>>,
    /// The installation contexts the command is available from.
    #[builder(default = "None")]
    pub integration_types: Option<Cow<'static, [InstallationContext]>>,
    /// The default set of permissions required for the command.
    ///
    /// Servers can edit these permissions individually for users/roles/channels
    /// in the server integration tab.
    #[builder(default = "None")]
    pub default_member_permissions: Option<Permissions>,
    /// Whether the command is only available in nsfw channels.
    #[builder(default = "false")]
    pub nsfw: bool,
    /// The root command option.
    pub data: CommandOption,
}

/// Contains the data for a command or command-group.
#[derive(Debug, Clone, ConstBuilder)]
#[non_exhaustive]
pub struct CommandOption {
    /// The name of the command.
    pub name: Cow<'static, str>,
    /// The description of the command.
    ///
    /// Required for chat-input commands, must be empty for context menu
    /// commands.
    #[builder(default = "Cow::Borrowed(\"\")")]
    pub description: Cow<'static, str>,
    /// The data for the command or group.
    pub data: CommandOptionData,
}

/// This represents a command option, that is either a command group or an
/// invokable command.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum CommandOptionData {
    /// A group with further sub-commands.
    Group(GroupData),
    /// An invokable command.
    Command(SubCommandData),
}

/// A group of commands.
#[derive(Debug, Clone, ConstBuilder)]
#[non_exhaustive]
pub struct GroupData {
    /// The sub-commands of this group.
    pub sub_commands: Cow<'static, [CommandOption]>,
}

/// A sub-command that may be nested in a group.
/// Also used to represent the invokable information about a root command.
#[derive(Debug, Clone, ConstBuilder)]
#[non_exhaustive]
pub struct SubCommandData {
    /// Logic to run when this command is invoked.
    ///
    /// This also determines whether this is a chat-input or a context menu
    /// command.
    pub invoke: Invoke,
    /// The chat-input command parameters.
    ///
    /// Must be empty for context menu commands.
    #[builder(default = "Cow::Borrowed(&[])")]
    pub parameters: Cow<'static, [Parameter]>,
}

/// How the command can be invoked.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Invoke {
    /// An invocation via chat-input (aka a slash-command).
    ChatInput(ChatInputFn),
    /// An invocation via the user context menu.
    User(UserFn),
    /// An invocation via the message context menu.
    Message(MessageFn),
}

/// A command parameter.
#[derive(Debug, Clone, ConstBuilder)]
#[non_exhaustive]
pub struct Parameter {
    /// The name of the parameter.
    pub name: Cow<'static, str>,
    /// The description for the parameter.
    pub description: Cow<'static, str>,
    /// Whether the parameter is required.
    pub required: bool,
    /// The autocomplete suggestion logic for this parameter.
    ///
    /// This is [`None`] if this parameter isn't autocompletable.
    #[builder(default = "None")]
    pub autocomplete: Option<AutocompleteFn>,
    /// A function pointer that returns the possible choices.
    ///
    /// Choices aren't "matched" by name but by index in this list. That is, it
    /// is expected that a choice parameter will be an integer parameter and
    /// that you can derive the intended value from said integer.
    ///
    /// This is only called to generate the options for Discord and isn't
    /// checked when the command is invoked, so it is acceptable to be somewhat
    /// expensive to run.
    //
    // this is a function pointer to allow filling this field from a trait in a const-context. while
    // a bit awkward as an api, this does also mean it's possible to lazily generate these choices
    // as needed.
    #[builder(default = "|| Cow::Borrowed(&[])")]
    pub choices: fn() -> Cow<'static, [Choice]>,
    /// A function pointer that sets required type information for the command
    /// option.
    ///
    /// The only field required to be set by this function is `kind`. It may
    /// also specify things such as numeric or input length limits.
    //
    // this is also a function pointer for the same reason as above.
    pub type_setter: fn(CreateCommandOption<'_>) -> CreateCommandOption<'_>,
}

/// A choice value for a command parameter.
#[derive(Debug, Clone, ConstBuilder)]
#[non_exhaustive]
pub struct Choice {
    /// The display name of the choice.
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

        // hitting an overflow here would require more memory
        // than is even addressable on a 64-bit system
        for (index, choice) in (0i64..).zip((self.choices)().iter()) {
            option = option.add_int_choice(choice.name.clone(), index);
        }

        (self.type_setter)(option)
    }
}
