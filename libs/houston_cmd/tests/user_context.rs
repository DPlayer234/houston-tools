#![expect(unused_crate_dependencies)]
use std::borrow::Cow;

use houston_cmd::model::*;
use houston_cmd::*;
use serenity::all::{InstallationContext, InteractionContext, Permissions, User};

#[test]
fn minimal_user_context() {
    #![allow(deprecated, reason = "macro emits deprecations as warnings")]

    #[context_command(user, name = "Just a command")]
    async fn just_a_command(_ctx: Context<'_>, _target: &User) -> anyhow::Result<()> {
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
                        invoke: Invoke::User(_),
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
fn maximal_user_context() {
    #[context_command(
        user,
        name = "Just a command",
        contexts = "Guild",
        integration_types = "User | Guild",
        default_member_permissions = "SEND_MESSAGES | VIEW_CHANNEL",
        nsfw = true,
        crate = "houston_cmd"
    )]
    async fn just_a_command(_ctx: Context<'_>, _target: &User) -> anyhow::Result<()> {
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
                        invoke: Invoke::User(_),
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
