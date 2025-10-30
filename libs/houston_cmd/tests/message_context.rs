#![expect(unused_crate_dependencies)]
use std::borrow::Cow;

use houston_cmd::model::*;
use houston_cmd::*;
use serenity::all::{InstallationContext, InteractionContext, Message, Permissions};

#[test]
fn minimal_message_context() {
    #![allow(deprecated, reason = "macro emits deprecations as warnings")]

    #[context_command(message, name = "Just a command")]
    async fn just_a_command(_ctx: Context<'_>, _target: &Message) -> anyhow::Result<()> {
        Ok(())
    }

    let command = just_a_command();
    assert!(
        matches!(
            command,
            Command {
                contexts: None,
                integration_types: None,
                default_member_permissions: None,
                nsfw: false,
                data: CommandOption {
                    name: Cow::Borrowed("Just a command"),
                    description: Cow::Borrowed(""),
                    data: CommandOptionData::Command(SubCommandData {
                        invoke: Invoke::Message(_),
                        parameters: Cow::Borrowed([]),
                        ..
                    }),
                    ..
                },
                ..
            }
        ),
        "{command:?}"
    );
}

#[test]
fn maximal_message_context() {
    #[context_command(
        message,
        name = "Just a command",
        contexts = "Guild",
        integration_types = "User | Guild",
        default_member_permissions = "SEND_MESSAGES | VIEW_CHANNEL",
        nsfw = true,
        crate = "houston_cmd"
    )]
    async fn just_a_command(_ctx: Context<'_>, _target: &Message) -> anyhow::Result<()> {
        Ok(())
    }

    const PERMISSIONS: Permissions = Permissions::SEND_MESSAGES.union(Permissions::VIEW_CHANNEL);
    let command = just_a_command();
    assert!(
        matches!(
            command,
            Command {
                contexts: Some(Cow::Borrowed([InteractionContext::Guild])),
                integration_types: Some(Cow::Borrowed([
                    InstallationContext::User,
                    InstallationContext::Guild
                ])),
                default_member_permissions: Some(PERMISSIONS),
                nsfw: true,
                data: CommandOption {
                    name: Cow::Borrowed("Just a command"),
                    description: Cow::Borrowed(""),
                    data: CommandOptionData::Command(SubCommandData {
                        invoke: Invoke::Message(_),
                        parameters: Cow::Borrowed([]),
                        ..
                    }),
                    ..
                },
                ..
            }
        ),
        "{command:?}"
    );
}
