use serenity::model::application::{
    CommandData, CommandDataOption, CommandDataOptionValue, CommandDataResolved, ResolvedValue,
};
use serenity::model::id::{AttachmentId, GenericChannelId, RoleId, UserId};

use super::ResolvedOption;

/// Internal helper to resolve options for a command.
///
/// This essentially just exists to save a few allocations done by serenity's
/// built-in command data resolution.
///
/// Call [`Self::sub_command`] until you receive [`None`] to descend into the
/// tree and find the right command, then call [`Self::options`] to resolve the
/// arguments to the command.
pub struct CommandOptionResolver<'a> {
    opts: &'a [CommandDataOption],
    resolved: &'a CommandDataResolved,
}

impl<'a> CommandOptionResolver<'a> {
    pub fn new(data: &'a CommandData) -> Self {
        Self {
            opts: &data.options,
            resolved: &data.resolved,
        }
    }

    /// Tries to descend into next sub command or sub command group. If another
    /// is found, returns [`Some`] with the command name.
    ///
    /// If there are no further sub commands specified, returns [`None`].
    pub fn sub_command(&mut self) -> Option<&'a str> {
        let cmd = self.opts.first()?;
        match &cmd.value {
            CommandDataOptionValue::SubCommand(opts)
            | CommandDataOptionValue::SubCommandGroup(opts) => {
                self.opts = opts;
                Some(&cmd.name)
            },
            _ => None,
        }
    }

    /// Resolves the options for the command. This is supposed to be called
    /// after descending the tree.
    ///
    /// Returns an error if there are further sub commands or if a command value
    /// must be rejected.
    pub fn options(self) -> Result<Box<[ResolvedOption<'a>]>, &'static str> {
        let Self { resolved, opts } = self;
        opts.iter()
            .map(|o| {
                #[warn(clippy::wildcard_enum_match_arm)]
                let value = match &o.value {
                    CommandDataOptionValue::SubCommand(_) => {
                        return Err("SubCommand cannot be an argument");
                    },
                    CommandDataOptionValue::SubCommandGroup(_) => {
                        return Err("SubCommandGroup cannot be an argument");
                    },
                    CommandDataOptionValue::Autocomplete { kind, value } => {
                        ResolvedValue::Autocomplete { kind: *kind, value }
                    },
                    CommandDataOptionValue::Boolean(v) => ResolvedValue::Boolean(*v),
                    CommandDataOptionValue::Integer(v) => ResolvedValue::Integer(*v),
                    CommandDataOptionValue::Number(v) => ResolvedValue::Number(*v),
                    CommandDataOptionValue::String(v) => ResolvedValue::String(v),
                    CommandDataOptionValue::Attachment(id) => resolve_attachment(resolved, *id)
                        .ok_or("attachment could not be resolved")?,
                    CommandDataOptionValue::Channel(id) => {
                        resolve_channel(resolved, *id).ok_or("channel could not be resolved")?
                    },
                    CommandDataOptionValue::User(id) => {
                        resolve_user(resolved, *id).ok_or("user could not be resolved")?
                    },
                    CommandDataOptionValue::Role(id) => {
                        resolve_role(resolved, *id).ok_or("role could not be resolved")?
                    },
                    CommandDataOptionValue::Mentionable(id) => {
                        resolve_user(resolved, UserId::new(id.get()))
                            .or_else(|| resolve_channel(resolved, GenericChannelId::new(id.get())))
                            .or_else(|| resolve_role(resolved, RoleId::new(id.get())))
                            .ok_or("mentionable could not be resolved")?
                    },
                    CommandDataOptionValue::Unknown(_) => {
                        return Err("Unknown value kind is not supported");
                    },
                    _ => return Err("unexpected CommandDataOptionValue variant"),
                };

                Ok(ResolvedOption {
                    name: &o.name,
                    value,
                })
            })
            .collect()
    }
}

fn resolve_attachment(
    resolved: &CommandDataResolved,
    id: AttachmentId,
) -> Option<ResolvedValue<'_>> {
    resolved.attachments.get(&id).map(ResolvedValue::Attachment)
}

fn resolve_channel(
    resolved: &CommandDataResolved,
    id: GenericChannelId,
) -> Option<ResolvedValue<'_>> {
    resolved.channels.get(&id).map(ResolvedValue::Channel)
}

fn resolve_user(resolved: &CommandDataResolved, id: UserId) -> Option<ResolvedValue<'_>> {
    let user = resolved.users.get(&id)?;
    let member = resolved.members.get(&id);
    Some(ResolvedValue::User(user, member))
}

fn resolve_role(resolved: &CommandDataResolved, id: RoleId) -> Option<ResolvedValue<'_>> {
    resolved.roles.get(&id).map(ResolvedValue::Role)
}
