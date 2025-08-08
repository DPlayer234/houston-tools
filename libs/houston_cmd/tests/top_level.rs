use std::borrow::Cow;

use houston_cmd::model::*;
use houston_cmd::*;
use serenity::all::{
    CreateAutocompleteResponse, InstallationContext, InteractionContext, Permissions,
};

#[test]
fn minimal_top_level() {
    #![allow(deprecated, reason = "macro emits deprecations as warnings")]

    /// Just a command.
    #[chat_command]
    async fn just_a_command(_ctx: Context<'_>) -> anyhow::Result<()> {
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
                    name: Cow::Borrowed("just_a_command"),
                    description: Cow::Borrowed("Just a command."),
                    data: CommandOptionData::Command(SubCommandData {
                        parameters: Cow::Borrowed([]),
                        invoke: Invoke::ChatInput(_),
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
fn maximal_top_level() {
    async fn autocomplete_count<'a>(
        _ctx: Context<'a>,
        _partial: &'a str,
    ) -> CreateAutocompleteResponse<'a> {
        unimplemented!()
    }

    /// Just a command.
    #[chat_command(
        name = "just-a-command",
        contexts = "Guild",
        integration_types = "User | Guild",
        default_member_permissions = "SEND_MESSAGES | VIEW_CHANNEL",
        nsfw = true,
        crate = "houston_cmd"
    )]
    async fn just_a_command(
        _ctx: Context<'_>,
        /// How much.
        #[name = "count"]
        #[autocomplete = "autocomplete_count"]
        _count: u32,
        /// Extras?
        #[name = "extra"]
        _extra: Option<bool>,
    ) -> anyhow::Result<()> {
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
                    name: Cow::Borrowed("just-a-command"),
                    description: Cow::Borrowed("Just a command."),
                    data: CommandOptionData::Command(SubCommandData {
                        invoke: Invoke::ChatInput(_),
                        parameters: Cow::Borrowed([
                            Parameter {
                                name: Cow::Borrowed("count"),
                                description: Cow::Borrowed("How much."),
                                required: true,
                                autocomplete: Some(_),
                                ..
                            },
                            Parameter {
                                name: Cow::Borrowed("extra"),
                                description: Cow::Borrowed("Extras?"),
                                required: false,
                                autocomplete: None,
                                ..
                            }
                        ]),
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
