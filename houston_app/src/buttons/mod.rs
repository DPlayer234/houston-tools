use std::mem::swap;
use std::{fmt, ptr};

use houston_btn::{AnyContext, AnyInteraction, ButtonContext, ErrorContext, Hooks, ModalContext};
pub use houston_btn::{ButtonAction, ButtonValue, EventHandler};

use crate::fmt::discord::interaction_location;
use crate::modules::core::buttons::Noop;
use crate::prelude::*;

#[cfg(test)]
mod tests;

pub mod prelude {
    pub use bson_model::ModelDocument as _;
    pub use const_builder::ConstBuilder;
    pub use houston_btn::{
        ButtonContext, ButtonReply, ButtonValue, ModalContext, Nav, button_value,
    };
    pub use serde::{Deserialize, Serialize};
    pub use serde_with::As;

    pub use super::{ButtonValueExt as _, ContextExt as _};
    pub use crate::helper::discord::components::*;
    pub use crate::prelude::*;
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

pub struct EventHandlerHooks;

// less generic interaction logging
fn log_interaction<I: AnyInteraction>(kind: &str, interaction: &I, args: &dyn fmt::Debug) {
    log::info!(
        "[{kind}] {}, {}: {args:?}",
        interaction_location(interaction.guild_id(), interaction.channel()),
        interaction.user().name,
    );
}

#[serenity::async_trait]
impl Hooks for EventHandlerHooks {
    async fn handle_error(&self, ctx: ErrorContext<'_>, err: anyhow::Error) {
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
        if let Err(res) = ctx.reply(reply).await {
            log::warn!("Error sending component error: {res}");
        }
    }

    fn on_button(&self, ctx: ButtonContext<'_>, args: &dyn fmt::Debug) {
        log_interaction("Button", ctx.interaction(), args);
    }

    fn on_modal(&self, ctx: ModalContext<'_>, args: &dyn fmt::Debug) {
        log_interaction("Modal", ctx.interaction(), args);
    }
}

impl<T: ButtonValue> ButtonValueExt for T {}

/// Provides the shared surface for values that can be used as button actions
/// and custom IDs.
pub trait ButtonValueExt: ButtonValue {
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
            #[expect(clippy::cast_possible_truncation)]
            let sentinel_key = ptr::from_ref(field_ref).addr() as u16;

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

/// Extension trait for the button context.
pub trait ContextExt<'a> {
    /// Gets the ref to the [`HBotData`] in the context.
    #[must_use]
    fn data_ref(self) -> &'a HBotData;
}

impl<'a, I: ?Sized + AnyInteraction> ContextExt<'a> for AnyContext<'a, I> {
    fn data_ref(self) -> &'a HBotData {
        self.serenity().data_ref::<HContextData>()
    }
}
