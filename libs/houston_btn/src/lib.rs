use std::fmt;

use anyhow::Context as _;
use extract_map::{ExtractKey, ExtractMap};
use serenity::futures::future::BoxFuture;
use serenity::gateway::client::FullEvent;
use serenity::model::application::{ComponentInteraction, Interaction, ModalInteraction};
use serenity::prelude::*;

use crate::context::ContextInner;
pub use crate::context::{AnyContext, AnyInteraction, ButtonContext, ErrorContext, ModalContext};
pub use crate::nav::Nav;

pub mod builtins;
mod context;
pub mod encoding;
mod nav;
#[doc(hidden)]
pub mod private;
#[cfg(test)]
mod tests;

pub mod prelude {
    pub use crate::{ButtonContext, ButtonReply, ButtonValue, ModalContext, Nav, button_value};
}

type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

#[serenity::async_trait]
pub trait Hooks: Send + Sync + 'static {
    async fn handle_error(&self, _ctx: ErrorContext<'_>, _err: anyhow::Error) {}
    fn on_button(&self, _ctx: ButtonContext<'_>, _args: &dyn fmt::Debug) {}
    fn on_modal(&self, _ctx: ModalContext<'_>, _args: &dyn fmt::Debug) {}
}

/// Event handler for custom button menus.
pub struct EventHandler {
    actions: ExtractMap<usize, ButtonAction>,
    hooks: Option<Box<dyn Hooks>>,
}

impl EventHandler {
    /// Create a new handler with the given button actions.
    ///
    /// # Errors
    ///
    /// Returns `Err` if any of the actions have the same key.
    pub fn new(actions: impl IntoIterator<Item = ButtonAction>) -> Result<Self> {
        let mut map = ExtractMap::new();
        for action in actions {
            let key = action.key;
            anyhow::ensure!(
                map.insert(action).is_none(),
                "duplicate button action for key `{key}`"
            );
        }

        Ok(Self {
            actions: map,
            hooks: None,
        })
    }

    fn action(&self, key: usize) -> Result<&ButtonAction> {
        self.actions.get(&key).context("unknown button action")
    }

    #[must_use]
    pub fn hooks(mut self, handler: Box<dyn Hooks>) -> Self {
        self.hooks = Some(handler);
        self
    }

    pub fn dispatch<'a>(
        &'a self,
        ctx: &'a Context,
        event: &'a FullEvent,
    ) -> Option<BoxFuture<'a, ()>> {
        match event {
            FullEvent::InteractionCreate {
                interaction: Interaction::Component(interaction),
                ..
            } => Some(Box::pin(self.dispatch_component(ctx, interaction))),
            FullEvent::InteractionCreate {
                interaction: Interaction::Modal(interaction),
                ..
            } => Some(Box::pin(self.dispatch_modal(ctx, interaction))),
            _ => None,
        }
    }

    /// Dispatches component interactions.
    pub async fn dispatch_component(&self, ctx: &Context, interaction: &ComponentInteraction) {
        let inner = ContextInner::new(self);
        if let Err(err) = Self::handle_component(ctx, interaction, &inner).await {
            Self::handle_dispatch_error(ctx, interaction, &inner, err).await
        }
    }

    /// Handles the component interaction dispatch.
    async fn handle_component(
        ctx: &Context,
        interaction: &ComponentInteraction,
        inner: &ContextInner<'_>,
    ) -> Result {
        let mut buf = encoding::StackBuf::new();
        let mut decoder = encoding::decode_custom_id(&mut buf, &interaction.data.custom_id)?;
        let key = decoder.read_key()?;
        let action = inner.state.action(key)?;

        let ctx = ButtonContext::new(ctx, interaction, inner);
        (action.invoke_button)(ctx, decoder).await
    }

    /// Dispatches modal interactions.
    pub async fn dispatch_modal(&self, ctx: &Context, interaction: &ModalInteraction) {
        let inner = ContextInner::new(self);
        if let Err(err) = Self::handle_modal(ctx, interaction, &inner).await {
            Self::handle_dispatch_error(ctx, interaction, &inner, err).await
        }
    }

    /// Handles the modal interaction dispatch.
    async fn handle_modal(
        ctx: &Context,
        interaction: &ModalInteraction,
        inner: &ContextInner<'_>,
    ) -> Result {
        let mut buf = encoding::StackBuf::new();
        let mut decoder = encoding::decode_custom_id(&mut buf, &interaction.data.custom_id)?;
        let key = decoder.read_key()?;
        let action = inner.state.action(key)?;

        let ctx = ModalContext::new(ctx, interaction, inner);
        (action.invoke_modal)(ctx, decoder).await
    }

    #[cold]
    async fn handle_dispatch_error(
        ctx: &Context,
        interaction: &dyn AnyInteraction,
        inner: &ContextInner<'_>,
        err: anyhow::Error,
    ) {
        if let Some(hooks) = inner.state.hooks.as_deref() {
            let ctx = ErrorContext::new(ctx, interaction, inner);
            hooks.handle_error(ctx, err).await;
        } else {
            log::error!("Dispatching event failed: {err:?}");
        }
    }
}

/// Provides the shared surface for values that can be used as button actions
/// and custom IDs.
///
/// Use [`button_value`] to implement this trait.
pub trait ButtonValue: Send + Sync {
    /// Gets an action that can be registered to the [`EventHandler`].
    //
    // note: for places that need the key, make sure to use
    // `const { T::ACTION.key }` for shorter code gen. beyond
    // me why that changes anything at all.
    const ACTION: ButtonAction;

    /// Converts this instance to a [`Nav`].
    #[must_use]
    fn to_nav(&self) -> Nav<'_>;

    /// Converts this instance to a component custom ID.
    #[must_use]
    fn to_custom_id(&self) -> String {
        self.to_nav().to_custom_id()
    }
}

/// Provides a way for button arguments to reply to the interaction.
pub trait ButtonReply: Sized + Send {
    /// Replies to the component interaction.
    fn reply(self, ctx: ButtonContext<'_>) -> impl Future<Output = Result> + Send;

    /// Replies to the modal interaction.
    fn modal_reply(self, ctx: ModalContext<'_>) -> impl Future<Output = Result> + Send {
        async fn unsupported() -> Result {
            anyhow::bail!("this button args type does not support modals");
        }

        _ = ctx;
        unsupported()
    }
}

/// Button action to be registered to the [`EventHandler`].
///
/// This is similar to what commands do, just for buttons and modals.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ButtonAction {
    /// The corresponding key used to identify this action.
    ///
    /// The same key is used for serialization by the action type.
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
    ok(dummy::<ButtonContext<'_>>().defer(true));
    ok(dummy::<ButtonContext<'_>>().reply(dummy()));
    ok(dummy::<ButtonContext<'_>>().edit(dummy()));
    ok(dummy::<ButtonContext<'_>>().modal(dummy()));

    ok(dummy::<ModalContext<'_>>());
    ok(dummy::<ModalContext<'_>>().acknowledge());
    ok(dummy::<ModalContext<'_>>().defer(true));
    ok(dummy::<ModalContext<'_>>().reply(dummy()));
    ok(dummy::<ModalContext<'_>>().edit(dummy()));
}
