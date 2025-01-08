//! Provides a slash-command only framework for use with [`serenity`].
//!
//! The data model and attributes _fairly_ closely model what Discord expects.
//! The framework _does not_ automatically register commands for you.
//!
//! The most basic example would look like this:
//!
//! ```no_run
//! use houston_cmd::*;
//!
//! /// Echoes your message back at you.
//! #[chat_command]
//! async fn echo(
//!     ctx: Context<'_>,
//!     /// The message to return.
//!     text: &str,
//! ) -> Result<(), serenity::Error> {
//!     let reply = CreateReply::new()
//!         .content(text);
//!
//!     ctx.send(reply).await?;
//!     Ok(())
//! }
//!
//! // in your main, construct a framework with the commands
//! // and add it to your serenity Client builder
//! let framework = Framework::new()
//!     .commands([echo()]);
//! ```
//!
//! The magic that's happening here is that it transforms the function you've
//! written into one that simply returns the command tree. This function is
//! guaranteed to be `const`.
//!
//! The doc-string on the `#[chat_command]` is required and is used as the
//! description. Similarly, doc-strings are required on every parameter other
//! than the context.
//!
//! Context menu commands can be created similarly:
//!
//! ```no_run
//! # use houston_cmd::*;
//! # use serenity::all::User;
//! #[context_command(
//!     user,
//!     name = "User Profile",
//! )]
//! async fn profile(
//!     ctx: Context<'_>,
//!     user: &User,
//! ) -> Result<(), serenity::Error> {
//!     todo!()
//! }
//! ```
//!
//! You must specify either `user` or `message` in the attribute, specifying
//! which context to register the command to. The `name` is required for context
//! menu commands, but the doc-strings aren't since context menu commands cannot
//! have descriptions.
//!
//! Chat commands can also be used to create groups:
//!
//! ```no_run
//! # use houston_cmd::*;
//! # use serenity::all::PartialMember;
//! /// Admin commands.
//! #[chat_command(
//!     contexts = "Guild",
//!     integration_types = "Guild",
//! )]
//! mod admin {
//!     /// Bans a server member.
//!     #[sub_command]
//!     async fn ban(
//!         ctx: Context<'_>,
//!         /// The member to ban.
//!         user: &PartialMember,
//!     ) -> Result<(), serenity::Error> {
//!         todo!()
//!     }
//!
//!     /// Kicks a server member.
//!     #[sub_command]
//!     async fn kick(
//!         ctx: Context<'_>,
//!         /// The member to kick.
//!         user: &PartialMember,
//!     ) -> Result<(), serenity::Error> {
//!         todo!()
//!     }
//! }
//! ```
//!
//! While this abuses the `mod` syntax, this still simply emits a function that
//! returns the command tree. As such, this `mod` actually inherits its `super`
//! scope, as if it contained `use super::*`.
//!
//! Sub-commands have to be attributed with `#[sub_command]` and may also be
//! nested groups. Items other than sub-commands and `use` items are not allowed
//! directly inside a group.
//!
//! Additionally, as the last example showed, you may specify additional values
//! in `#[chat_command]` and `#[context_command]`:
//!
//! | Name                         | Meaning |
//! |:---------------------------- |:------- |
//! | `name`                       | Replaces the display name. Required for context commands. |
//! | `default_member_permissions` | A `\|` separated list of [`Permissions`] values. Specifies the default set of required permissions for the command. |
//! | `contexts`                   | A `\|` separated list of [`InteractionContext`] values in which the command can be used. |
//! | `integration_types`          | A `\|` separated list of [`InstallationContext`] values in which the command can be used. |
//! | `nsfw`                       | Indicates that the command can only be used in nsfw channels. |
//!
//! For `#[sub_command]`, the following values can be specified:
//!
//! | Name                         | Meaning |
//! |:---------------------------- |:------- |
//! | `name`                       | Replaces the display name. |
//!
//! Further, parameters to `#[chat_command]` functions can have the following
//! attributes applied:
//!
//! | Name                      | Meaning |
//! |:------------------------- |:------- |
//! | `autocomplete`            | The path to a function to be used for autocompletion. |
//! | `min`/`max`               | Numeric limits to the input value. |
//! | `min_length`/`max_length` | Limits to the length of the input. |
//!
//! [`InteractionContext`]: serenity::model::application::InteractionContext
//! [`InstallationContext`]: serenity::model::application::InstallationContext
//! [`Permissions`]: serenity::model::Permissions

mod args;
mod context;
mod error;
mod framework;
mod macros;
pub mod model;
#[doc(hidden)]
pub mod private;
mod reply;

pub use ::houston_cmd_macros::{chat_command, context_command};
pub use args::{ChoiceArg, MessageContextArg, SlashArg, UserContextArg};
pub use context::Context;
pub use error::Error;
pub use framework::Framework;
pub use reply::{CreateReply, EditReply, ReplyHandle};

pub type BoxFuture<'a, T> = serenity::futures::future::BoxFuture<'a, T>;

/// Converts an iterator of commands into create-command payloads to be
/// registered to Discord.
pub fn to_create_command<'a>(
    commands: impl IntoIterator<Item = &'a model::Command>,
) -> Vec<serenity::builder::CreateCommand<'static>> {
    commands
        .into_iter()
        .map(|c| c.to_create_command())
        .collect()
}

/// Mostly a sanity-check to ensure that this crate is usable.
fn _assert_traits() {
    fn send<T: Send>(_: T) {}
    fn send_sync<T: Send + Sync>(_: T) {}
    fn dummy<T>() -> T {
        unreachable!()
    }

    send(dummy::<CreateReply<'_>>());
    send(dummy::<EditReply<'_>>());
    send(dummy::<ReplyHandle<'_>>());
    send_sync(dummy::<Context<'_>>());
    send_sync(dummy::<Error<'_>>());
    send_sync(dummy::<Framework>());

    send(dummy::<Context<'_>>().defer(dummy()));
    send(dummy::<Context<'_>>().send(dummy()));
}
