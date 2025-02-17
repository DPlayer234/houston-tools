use serenity::model::prelude::*;

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
    pub fn options(self) -> Result<Vec<ResolvedOption<'a>>, &'static str> {
        let resolved = self.resolved;
        self.opts
            .iter()
            .map(|o| {
                let value = match &o.value {
                    CommandDataOptionValue::SubCommand(_) => {
                        return Err("SubCommand cannot be an argument")
                    },
                    CommandDataOptionValue::SubCommandGroup(_) => {
                        return Err("SubCommandGroup cannot be an argument")
                    },
                    CommandDataOptionValue::Autocomplete { kind, value } => {
                        ResolvedValue::Autocomplete { kind: *kind, value }
                    },
                    CommandDataOptionValue::Boolean(v) => ResolvedValue::Boolean(*v),
                    CommandDataOptionValue::Integer(v) => ResolvedValue::Integer(*v),
                    CommandDataOptionValue::Number(v) => ResolvedValue::Number(*v),
                    CommandDataOptionValue::String(v) => ResolvedValue::String(v),
                    CommandDataOptionValue::Attachment(id) => resolved
                        .attachments
                        .get(id)
                        .map(ResolvedValue::Attachment)
                        .ok_or("attachment could not be resolved")?,
                    CommandDataOptionValue::Channel(id) => resolved
                        .channels
                        .get(id)
                        .map(ResolvedValue::Channel)
                        .ok_or("channel could not be resolved")?,
                    CommandDataOptionValue::User(id) => resolved
                        .users
                        .get(id)
                        .map(|u| ResolvedValue::User(u, resolved.members.get(id)))
                        .ok_or("user could not be resolved")?,
                    CommandDataOptionValue::Role(id) => resolved
                        .roles
                        .get(id)
                        .map(ResolvedValue::Role)
                        .ok_or("role could not be resolved")?,
                    CommandDataOptionValue::Mentionable(_) => {
                        return Err("Mentionable is not supported")
                    },
                    CommandDataOptionValue::Unknown(_) => {
                        return Err("Unknown value kind is not supported")
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
