use std::mem::swap;
use std::sync::atomic::AtomicBool;
use std::{fmt, ptr};

use extract_map::{ExtractKey, ExtractMap};
use houston_cmd::BoxFuture;
use serde::Deserialize as _;
use serenity::prelude::*;

use crate::fmt::discord::interaction_location;
use crate::modules::core::buttons::Noop;
use crate::prelude::*;

mod context;
pub mod encoding;
mod nav;
#[cfg(test)]
mod test;

pub use context::{AnyContext, AnyInteraction, ButtonContext, ModalContext};
pub use nav::Nav;

pub mod prelude {
    pub use bson_model::ModelDocument as _;

    pub(crate) use super::button_value;
    pub use super::{ButtonContext, ButtonReply, ButtonValue, ModalContext, Nav};
    pub use crate::prelude::*;
}

/// Event handler for custom button menus.
pub struct EventHandler {
    actions: ExtractMap<usize, ButtonAction>,
}

crate::modules::impl_handler!(EventHandler, |t, ctx| match _ {
    FullEvent::InteractionCreate {
        interaction: Interaction::Component(interaction),
        ..
    } => t.dispatch_component(ctx, interaction),
    FullEvent::InteractionCreate {
        interaction: Interaction::Modal(interaction),
        ..
    } => t.dispatch_modal(ctx, interaction),
});

pub fn log_interaction<I: AnyInteraction>(kind: &str, interaction: &I, args: &dyn fmt::Debug) {
    log::info!(
        "[{kind}] {}, {}: {args:?}",
        interaction_location(interaction.guild_id(), interaction.channel()),
        interaction.user().name,
    );
}

impl EventHandler {
    /// Create a new handler with the given button actions.
    pub fn new(actions: impl IntoIterator<Item = ButtonAction>) -> Result<Self> {
        let mut map = ExtractMap::new();
        for action in actions {
            let key = action.key;
            anyhow::ensure!(
                map.insert(action).is_none(),
                "duplicate button action for key `{key}`"
            );
        }

        Ok(Self { actions: map })
    }

    /// Dispatches component interactions.
    async fn dispatch_component(&self, ctx: &Context, interaction: &ComponentInteraction) {
        let reply_state = AtomicBool::new(false);
        if let Err(err) = self.handle_component(ctx, interaction, &reply_state).await {
            Box::pin(self.handle_dispatch_error(
                ctx,
                interaction.id,
                &interaction.token,
                reply_state.into_inner(),
                err,
            ))
            .await
        }
    }

    /// Handles the component interaction dispatch.
    async fn handle_component(
        &self,
        ctx: &Context,
        interaction: &ComponentInteraction,
        reply_state: &AtomicBool,
    ) -> Result {
        use ComponentInteractionDataKind as Kind;

        let custom_id: &str = match &interaction.data.kind {
            Kind::StringSelect { values } if values.len() == 1 => &values[0],
            Kind::Button => &interaction.data.custom_id,
            _ => anyhow::bail!("invalid button interaction"),
        };

        let mut buf = encoding::StackBuf::new();
        let mut decoder = encoding::decode_custom_id(&mut buf, custom_id)?;
        let key = usize::deserialize(&mut decoder)?;
        let action = self.actions.get(&key).context("unknown button action")?;

        let ctx = ButtonContext {
            reply_state,
            serenity: ctx,
            interaction,
            data: ctx.data_ref::<HContextData>(),
        };

        (action.invoke_button)(ctx, decoder).await
    }

    /// Dispatches modal interactions.
    async fn dispatch_modal(&self, ctx: &Context, interaction: &ModalInteraction) {
        let reply_state = AtomicBool::new(false);
        if let Err(err) = self.handle_modal(ctx, interaction, &reply_state).await {
            Box::pin(self.handle_dispatch_error(
                ctx,
                interaction.id,
                &interaction.token,
                reply_state.into_inner(),
                err,
            ))
            .await
        }
    }

    /// Handles the modal interaction dispatch.
    async fn handle_modal(
        &self,
        ctx: &Context,
        interaction: &ModalInteraction,
        reply_state: &AtomicBool,
    ) -> Result {
        let mut buf = encoding::StackBuf::new();
        let mut decoder = encoding::decode_custom_id(&mut buf, &interaction.data.custom_id)?;
        let key = usize::deserialize(&mut decoder)?;
        let action = self.actions.get(&key).context("unknown button action")?;

        let ctx = ModalContext {
            reply_state,
            serenity: ctx,
            interaction,
            data: ctx.data_ref::<HContextData>(),
        };

        (action.invoke_modal)(ctx, decoder).await
    }

    #[cold]
    async fn handle_dispatch_error(
        &self,
        ctx: &Context,
        interaction_id: InteractionId,
        interaction_token: &str,
        reply_state: bool,
        err: anyhow::Error,
    ) {
        if let Some(ser_err) = err.downcast_ref::<serenity::Error>() {
            // print both errors to preserve the stack trace, if present
            log::warn!("Discord interaction error: {ser_err:?} / {err:?}");
            return;
        }

        let err_text = match err.downcast::<HArgError>() {
            Ok(err) => err.msg,
            Err(err) => {
                log::warn!("Component error: {err:?}");
                format!("Button error: ```{err}```").into()
            },
        };

        let embed = CreateEmbed::new()
            .description(err_text)
            .color(ERROR_EMBED_COLOR);

        let reply = CreateReply::new().ephemeral(true).embed(embed);

        let res = if reply_state {
            let response = reply.into_interaction_followup();
            response
                .execute(&ctx.http, None, interaction_token)
                .await
                .map(|_| ())
        } else {
            let response = reply.into_interaction_response();
            let response = CreateInteractionResponse::Message(response);
            response
                .execute(&ctx.http, interaction_id, interaction_token)
                .await
        };

        if let Err(res) = res {
            log::warn!("Error sending component error: {res}");
        }
    }
}

/// Provides the shared surface for values that can be used as button actions
/// and custom IDs.
///
/// Use [`button_value`] to implement this trait.
pub trait ButtonValue: Send + Sync {
    /// Unique key for this action type.
    const ACTION_KEY: usize;

    /// Gets an action that can be registered to the [`EventHandler`].
    #[must_use]
    fn action() -> ButtonAction;

    /// Converts this instance to a component custom ID.
    #[must_use]
    fn to_custom_id(&self) -> String;

    /// Converts this instance to a [`Nav`].
    #[must_use]
    fn to_nav(&self) -> Nav<'_>;

    /// Creates a new button that would switch to a state where one field is
    /// changed.
    ///
    /// If the field value is the same, instead returns a disabled button with
    /// the sentinel value.
    fn new_button<'a, T, F, S>(&mut self, field: F, value: T, sentinel: S) -> CreateButton<'a>
    where
        T: PartialEq,
        F: Fn(&mut Self) -> &mut T,
        S: FnOnce(T) -> u16,
    {
        let field_ref = field(self);
        let disabled = *field_ref == value;
        if disabled {
            // This value is intended to be unique for a given object.
            // It isn't used in any way other than as a discriminator.
            let sentinel_key = ptr::from_ref(field_ref) as u16;

            let sentinel = Noop::new(sentinel_key, sentinel(value));
            let custom_id = sentinel.to_custom_id();
            CreateButton::new(custom_id).disabled(true)
        } else {
            let custom_id = self.to_custom_id_with(field, value);
            CreateButton::new(custom_id)
        }
    }

    /// Creates a new select option that would switch to a state where one field
    /// is changed.
    fn new_select_option<'a, T, F>(
        &mut self,
        label: impl Into<Cow<'a, str>>,
        field: F,
        value: T,
    ) -> CreateSelectMenuOption<'a>
    where
        T: PartialEq,
        F: Fn(&mut Self) -> &mut T,
    {
        let default = *field(self) == value;
        let custom_id = self.to_custom_id_with(field, value);

        CreateSelectMenuOption::new(label, custom_id).default_selection(default)
    }

    /// Creates a custom ID with one field replaced.
    #[must_use]
    fn to_custom_id_with<T, F>(&mut self, field: F, mut value: T) -> String
    where
        F: Fn(&mut Self) -> &mut T,
    {
        // Swap new value into the field
        swap(field(self), &mut value);
        // Create the custom ID
        let custom_id = self.to_custom_id();
        // Move original value back into field, dropping the new value.
        *field(self) = value;

        custom_id
    }
}

/// Provides a way for button arguments to reply to the interaction.
pub trait ButtonReply: Sized + Send {
    /// Replies to the component interaction.
    async fn reply(self, ctx: ButtonContext<'_>) -> Result;

    /// Replies to the modal interaction.
    async fn modal_reply(self, ctx: ModalContext<'_>) -> Result {
        _ = ctx;
        anyhow::bail!("this button args type does not support modals");
    }
}

/// Button action to be registered to the [`EventHandler`].
///
/// This is similar to what commands do, just for buttons and modals.
#[derive(Debug, Clone, Copy)]
pub struct ButtonAction {
    /// The corresponding [`ButtonValue::ACTION_KEY`].
    pub key: usize,
    /// The function to invoke for buttons.
    pub invoke_button:
        for<'ctx> fn(ButtonContext<'ctx>, encoding::Decoder<'ctx>) -> BoxFuture<'ctx, Result>,
    /// The function to invoke for modals.
    pub invoke_modal:
        for<'ctx> fn(ModalContext<'ctx>, encoding::Decoder<'ctx>) -> BoxFuture<'ctx, Result>,
}

impl ExtractKey<usize> for ButtonAction {
    fn extract_key(&self) -> &usize {
        &self.key
    }
}

/// Implements the [`ButtonValue`] trait.
/// Accepts the type and its action key as a [`usize`].
///
/// The type in question needs to implement the following:
/// - [`ButtonReply`]
/// - [`fmt::Debug`]
/// - [`serde::Deserialize`]
/// - [`serde::Serialize`]
///
/// If the type has lifetimes, specify them as `'_`.
macro_rules! button_value {
    ($Ty:ty, $key:literal) => {
        impl $crate::buttons::ButtonValue for $Ty {
            const ACTION_KEY: usize = $key;

            fn action() -> $crate::buttons::ButtonAction {
                $crate::buttons::ButtonAction {
                    key: $key,
                    invoke_button: |ctx, mut buf| {
                        let this = buf.read_to_end::<$Ty>();
                        Box::pin(async move {
                            let this = this?;
                            $crate::buttons::log_interaction("Button", ctx.interaction, &this);
                            this.reply(ctx).await
                        })
                    },
                    invoke_modal: |ctx, mut buf| {
                        let this = buf.read_to_end::<$Ty>();
                        Box::pin(async move {
                            let this = this?;
                            $crate::buttons::log_interaction("Modal", ctx.interaction, &this);
                            this.modal_reply(ctx).await
                        })
                    },
                }
            }

            fn to_custom_id(&self) -> String {
                $crate::buttons::encoding::to_custom_id(self)
            }

            fn to_nav(&self) -> Nav<'_> {
                $crate::buttons::Nav::from_action_value(self)
            }
        }
    };
}

pub(crate) use button_value;

/// Compile-time helper to assert that types are [`Send`] as expected.
///
/// Only done so we get errors at an early point rather than a sporadic "future
/// is not send" elsewhere.
fn _assert_traits() {
    fn ok<T: Send>(_v: T) {}
    fn dummy<T>() -> T {
        unreachable!()
    }

    ok(dummy::<Nav<'_>>());

    ok(dummy::<ButtonContext<'_>>());
    ok(dummy::<ButtonContext<'_>>().acknowledge());
    ok(dummy::<ButtonContext<'_>>().defer_as(true));
    ok(dummy::<ButtonContext<'_>>().reply(dummy()));
    ok(dummy::<ButtonContext<'_>>().edit(dummy()));

    ok(dummy::<ModalContext<'_>>());
    ok(dummy::<ModalContext<'_>>().acknowledge());
    ok(dummy::<ModalContext<'_>>().defer_as(true));
    ok(dummy::<ModalContext<'_>>().reply(dummy()));
    ok(dummy::<ModalContext<'_>>().edit(dummy()));
}
