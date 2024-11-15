use poise::{Command, ContextMenuCommandAction};
use serde::Serialize;
use serenity::builder::CreateCommandOption;
use serenity::model::application::{CommandOptionType, CommandType, InstallationContext, InteractionContext};
use serenity::model::permissions::Permissions;

// Custom Create Command payload type to include new data
#[derive(Clone, Debug, Serialize)]
pub struct CustomCreateCommand {
    name: String,
    description: String,
    options: Vec<CreateCommandOption<'static>>,
    #[serde(skip_serializing_if = "Permissions::is_empty")]
    default_member_permissions: Permissions,
    #[serde(rename = "type")]
    kind: CommandType,
    contexts: Vec<InteractionContext>,
    integration_types: Vec<InstallationContext>,
    nsfw: bool,
}

// This is somewhat copied from poise's source code with some parts deleted.
// The idea here is that we need to specify the allowed contexts for commands, which is currently missing in both serenity and poise releases.
// If and when that is available, this entire file will be removed and we will just use the built-in command builder.

pub fn build_commands<E, U>(commands: &[Command<E, U>]) -> Vec<CustomCreateCommand> {
    fn recursively_add_context_menu_commands<U, E>(
        builder: &mut Vec<CustomCreateCommand>,
        command: &Command<U, E>,
    ) {
        if let Some(context_menu_command) = create_as_context_menu_command(command) {
            builder.push(context_menu_command);
        }
        for subcommand in &command.subcommands {
            recursively_add_context_menu_commands(builder, subcommand);
        }
    }

    let mut commands_builder = Vec::with_capacity(commands.len());
    for command in commands {
        if let Some(slash_command) = create_as_slash_command(command) {
            commands_builder.push(slash_command);
        }
        recursively_add_context_menu_commands(&mut commands_builder, command);
    }
    commands_builder
}

fn contexts<E, U>(cmd: &Command<E, U>) -> Vec<InteractionContext> {
    if cmd.guild_only {
        vec![InteractionContext::Guild]
    } else if cmd.dm_only {
        vec![InteractionContext::BotDm, InteractionContext::PrivateChannel]
    } else {
        vec![InteractionContext::Guild, InteractionContext::BotDm, InteractionContext::PrivateChannel]
    }
}

fn integration_types<E, U>(cmd: &Command<E, U>) -> Vec<InstallationContext> {
    if let Some(&ctx) = cmd.custom_data.downcast_ref::<&[InstallationContext]>() {
        return ctx.to_vec();
    }

    if cmd.guild_only {
        vec![InstallationContext::Guild]
    } else {
        vec![InstallationContext::Guild, InstallationContext::User]
    }
}

fn create_as_slash_command<E, U>(cmd: &Command<E, U>) -> Option<CustomCreateCommand> {
    cmd.slash_action?;

    let mut options = Vec::new();
    if cmd.subcommands.is_empty() {
        for param in &cmd.parameters {
            // Using `?` because if this command has slash-incompatible parameters, we cannot
            // just ignore them but have to abort the creation process entirely
            options.push(param.create_as_slash_command_option()?);
        }
    } else {
        for subcommand in &cmd.subcommands {
            if let Some(subcommand) = create_as_subcommand(subcommand) {
                options.push(subcommand);
            }
        }
    }

    Some(CustomCreateCommand {
        name: cmd.name.clone(),
        description: cmd.description.clone().unwrap_or_else(|| "---".to_owned()),
        options,
        default_member_permissions: cmd.default_member_permissions,
        kind: CommandType::ChatInput,
        contexts: contexts(cmd),
        integration_types: integration_types(cmd),
        nsfw: cmd.nsfw_only
    })
}

fn create_as_context_menu_command<E, U>(cmd: &Command<E, U>) -> Option<CustomCreateCommand> {
    let context_menu_action = cmd.context_menu_action?;

    let kind = match context_menu_action {
        ContextMenuCommandAction::User(_) => CommandType::User,
        ContextMenuCommandAction::Message(_) => CommandType::Message,
        _ => unreachable!(),
    };

    Some(CustomCreateCommand {
        name: cmd.context_menu_name.clone().unwrap_or_else(|| cmd.name.clone()),
        description: String::new(),
        options: Vec::new(),
        default_member_permissions: cmd.default_member_permissions,
        kind,
        contexts: contexts(cmd),
        integration_types: integration_types(cmd),
        nsfw: cmd.nsfw_only
    })
}

fn create_as_subcommand<E, U>(cmd: &Command<E, U>) -> Option<CreateCommandOption<'static>> {
    cmd.slash_action?;

    let kind = if cmd.subcommands.is_empty() {
        CommandOptionType::SubCommand
    } else {
        CommandOptionType::SubCommandGroup
    };

    let description = cmd.description.as_deref().unwrap_or("A slash command").to_owned();
    let mut builder = CreateCommandOption::new(kind, cmd.name.clone(), description);

    if cmd.subcommands.is_empty() {
        for param in &cmd.parameters {
            // Using `?` because if this command has slash-incompatible parameters, we cannot
            // just ignore them but have to abort the creation process entirely
            builder = builder.add_sub_option(param.create_as_slash_command_option()?);
        }
    } else {
        for subcommand in &cmd.subcommands {
            if let Some(subcommand) = create_as_subcommand(subcommand) {
                builder = builder.add_sub_option(subcommand);
            }
        }
    }

    Some(builder)
}
