use std::mem::swap;
use std::ptr;

use serenity::prelude::*;
use utils::fields::FieldMut;

use crate::modules::{azur, core as core_mod, perks, starboard};
use crate::prelude::*;

mod context;
mod encoding;
#[cfg(test)]
mod test;

pub use context::{ButtonContext, ModalContext};

pub mod prelude {
    #[allow(unused_imports)]
    pub use super::{
        ButtonArgs, ButtonArgsRef, ButtonArgsReply, ButtonContext, ButtonMessage, CustomData,
        ModalContext, ToCustomData,
    };
    pub use crate::prelude::*;
}

/// Helper macro that repeats needed code for every [`ButtonArgs`] variant.
macro_rules! define_button_args {
    ($($(#[$attr:meta])* $name:ident($Ty:ty)),* $(,)?) => {
        /// The supported button interaction arguments.
        ///
        /// This is owned data that can be deserialized into.
        /// To serialize it, call [`ButtonArgs::borrow`] first.
        #[derive(Debug, Clone, PartialEq, serde::Deserialize)]
        pub enum ButtonArgs {
            $(
                $(#[$attr])*
                $name($Ty),
            )*
        }

        /// The supported button interaction arguments.
        ///
        /// This is borrowed data that can be serialized.
        #[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
        pub enum ButtonArgsRef<'a> {
            $(
                $(#[$attr])*
                $name(&'a $Ty),
            )*
        }

        $(
            impl From<$Ty> for ButtonArgs {
                fn from(value: $Ty) -> Self {
                    Self::$name(value)
                }
            }

            impl<'a> From<&'a $Ty> for ButtonArgsRef<'a> {
                fn from(value: &'a $Ty) -> Self {
                    Self::$name(value)
                }
            }
        )*

        impl ButtonArgs {
            /// Borrows the inner data.
            #[must_use]
            pub const fn borrow(&self) -> ButtonArgsRef<'_> {
                match self {
                    $(
                        Self::$name(v) => ButtonArgsRef::$name(v),
                    )*
                }
            }

            async fn reply(self, ctx: ButtonContext<'_>) -> Result {
                match self {
                    $(
                        Self::$name(args) => args.reply(ctx).await,
                    )*
                }
            }

            async fn modal_reply(self, ctx: ModalContext<'_>) -> Result {
                match self {
                    $(
                        Self::$name(args) => args.modal_reply(ctx).await,
                    )*
                }
            }
        }
    };
}

// to avoid unexpected effects for old buttons, don't insert new variants
// anywhere other than the bottom and don't reorder them!
define_button_args! {
    /// Unused button. A sentinel value is used to avoid duplicating custom IDs.
    None(core_mod::buttons::None),
    /// Open the ship detail view.
    AzurShip(azur::buttons::ship::View),
    /// Open the augment detail view.
    AzurAugment(azur::buttons::augment::View),
    /// Open the skill detail view.
    AzurSkill(azur::buttons::skill::View),
    /// Open the ship lines detail view.
    AzurLines(azur::buttons::lines::View),
    /// Open the ship filter list view.
    AzurSearchShip(azur::buttons::search_ship::View),
    /// Open the ship shadow equip details.
    AzurShadowEquip(azur::buttons::shadow_equip::View),
    /// Open the equipment details.
    AzurEquip(azur::buttons::equip::View),
    /// Open the equipment search.
    AzurSearchEquip(azur::buttons::search_equip::View),
    /// Open the augment search.
    AzurSearchAugment(azur::buttons::search_augment::View),
    /// Open the perk store.
    PerksStore(perks::buttons::shop::View),
    /// Open the starboard top view.
    StarboardTop(starboard::buttons::top::View),
    /// Open the starboard top posts view.
    StarboardTopPosts(starboard::buttons::top_posts::View),
    /// Open the "go to page" modal.
    ToPage(core_mod::buttons::ToPage),
    /// Delete the source message.
    Delete(core_mod::buttons::Delete),
    /// Sets the birthday for the perks module.
    PerksBirthdaySet(perks::buttons::birthday::Set),
    /// Open a Juustagram chat.
    AzurJuustagramChat(azur::buttons::juustagram_chat::View),
    /// Open the Juustagram chat search.
    AzurSearchJuustagramChat(azur::buttons::search_justagram_chat::View),
}

impl ButtonArgs {
    /// Constructs button arguments from a component custom ID.
    pub fn from_custom_id(id: &str) -> Result<Self> {
        encoding::from_custom_id(id)
    }
}

impl<'a> From<&'a ButtonArgs> for ButtonArgsRef<'a> {
    fn from(value: &'a ButtonArgs) -> Self {
        value.borrow()
    }
}

/// Event handler for custom button menus.
pub mod handler {
    use std::sync::atomic::AtomicBool;

    use super::*;

    /// To be called in [`EventHandler::interaction_create`].
    pub async fn interaction_create(ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Component(interaction) => dispatch_component(ctx, interaction).await,
            Interaction::Modal(interaction) => dispatch_modal(ctx, interaction).await,
            _ => {}, // we only handle component and modal interactions
        }
    }

    async fn dispatch_component(ctx: Context, interaction: ComponentInteraction) {
        let reply_state = AtomicBool::new(false);
        if let Err(err) = handle_component(&ctx, &interaction, &reply_state).await {
            handle_dispatch_error(
                ctx,
                interaction.id,
                &interaction.token,
                reply_state.into_inner(),
                err,
            )
            .await
        }
    }

    /// Handles the component interaction dispatch.
    async fn handle_component(
        ctx: &Context,
        interaction: &ComponentInteraction,
        reply_state: &AtomicBool,
    ) -> Result {
        use ComponentInteractionDataKind as Kind;

        let custom_id: &str = match &interaction.data.kind {
            Kind::StringSelect { values } if values.len() == 1 => &values[0],
            Kind::Button => &interaction.data.custom_id,
            _ => anyhow::bail!("Invalid interaction."),
        };

        let args = ButtonArgs::from_custom_id(custom_id)?;
        log::trace!("{}: {:?}", interaction.user.name, args);

        args.reply(ButtonContext {
            reply_state,
            serenity: ctx,
            interaction,
            data: ctx.data_ref::<HContextData>(),
        })
        .await
    }

    async fn dispatch_modal(ctx: Context, interaction: ModalInteraction) {
        let reply_state = AtomicBool::new(false);
        if let Err(err) = handle_modal(&ctx, &interaction, &reply_state).await {
            handle_dispatch_error(
                ctx,
                interaction.id,
                &interaction.token,
                reply_state.into_inner(),
                err,
            )
            .await
        }
    }

    /// Handles the modal interaction dispatch.
    async fn handle_modal(
        ctx: &Context,
        interaction: &ModalInteraction,
        reply_state: &AtomicBool,
    ) -> Result {
        let args = ButtonArgs::from_custom_id(&interaction.data.custom_id)?;
        log::trace!("{}: {:?}", interaction.user.name, args);

        args.modal_reply(ModalContext {
            reply_state,
            serenity: ctx,
            interaction,
            data: ctx.data_ref::<HContextData>(),
        })
        .await
    }

    #[cold]
    async fn handle_dispatch_error(
        ctx: Context,
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

/// Provides a way to convert an object into a component custom ID.
///
/// This is auto-implemented for every type held by [`ButtonArgs`].
pub trait ToCustomData {
    /// Converts this instance to a component custom ID.
    #[must_use]
    fn to_custom_id(&self) -> String;

    /// Converts this instance to custom data.
    #[must_use]
    fn to_custom_data(&self) -> CustomData;

    /// Creates a new button that would switch to a state where one field is
    /// changed.
    ///
    /// If the field value is the same, instead returns a disabled button with
    /// the sentinel value.
    fn new_button<'a, T: PartialEq>(
        &mut self,
        field: impl FieldMut<Self, T>,
        value: T,
        sentinel: impl FnOnce(T) -> u16,
    ) -> CreateButton<'a> {
        let disabled = *field.get(self) == value;
        if disabled {
            // This value is intended to be unique for a given object.
            // It isn't used in any way other than as a discriminator.
            let sentinel_key = ptr::from_ref(field.get(self)) as u16;

            let sentinel = core_mod::buttons::None::new(sentinel_key, sentinel(value));
            let custom_id = sentinel.to_custom_id();
            CreateButton::new(custom_id).disabled(true)
        } else {
            let custom_id = self.to_custom_id_with(field, value);
            CreateButton::new(custom_id)
        }
    }

    /// Creates a new select option that would switch to a state where one field
    /// is changed.
    fn new_select_option<'a, T: PartialEq>(
        &mut self,
        label: impl Into<Cow<'a, str>>,
        field: impl FieldMut<Self, T>,
        value: T,
    ) -> CreateSelectMenuOption<'a> {
        let default = *field.get(self) == value;
        let custom_id = self.to_custom_id_with(field, value);

        CreateSelectMenuOption::new(label, custom_id).default_selection(default)
    }

    /// Creates a custom ID with one field replaced.
    #[must_use]
    fn to_custom_id_with<T>(&mut self, field: impl FieldMut<Self, T>, mut value: T) -> String {
        // Swap new value into the field
        swap(field.get_mut(self), &mut value);
        // Create the custom ID
        let custom_id = self.to_custom_id();
        // Move original value back into field, dropping the new value.
        *field.get_mut(self) = value;

        custom_id
    }
}

impl<T> ToCustomData for T
where
    for<'a> &'a T: Into<ButtonArgsRef<'a>>,
{
    fn to_custom_id(&self) -> String {
        encoding::to_custom_id(self.into())
    }

    fn to_custom_data(&self) -> CustomData {
        CustomData::from_button_args(self.into())
    }
}

/// Provides a way for button arguments to reply to the interaction.
pub trait ButtonArgsReply: Sized {
    /// Replies to the component interaction.
    async fn reply(self, ctx: ButtonContext<'_>) -> Result;

    /// Replies to the modal interaction.
    async fn modal_reply(self, ctx: ModalContext<'_>) -> Result {
        _ = ctx;
        anyhow::bail!("this button args type does not support modals");
    }
}

/// Provides a way for button arguments to modify the create-reply payload.
pub trait ButtonMessage: Sized {
    /// Creates an edit-reply payload.
    fn edit_reply(self, ctx: ButtonContext<'_>) -> Result<EditReply<'_>>;

    /// Creates an edit-reply payload.
    fn edit_modal_reply(self, ctx: ModalContext<'_>) -> Result<EditReply<'_>> {
        _ = ctx;
        anyhow::bail!("this button args type does not support modals");
    }
}

impl<T: ButtonMessage> ButtonArgsReply for T {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        self.edit_reply(ctx.clone())?
            .execute_as_response(
                &ctx.serenity.http,
                ctx.interaction.id,
                &ctx.interaction.token,
            )
            .await?;
        Ok(())
    }

    async fn modal_reply(self, ctx: ModalContext<'_>) -> Result {
        self.edit_modal_reply(ctx.clone())?
            .execute_as_response(
                &ctx.serenity.http,
                ctx.interaction.id,
                &ctx.interaction.token,
            )
            .await?;
        Ok(())
    }
}

/// Represents custom data for another menu.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CustomData(encoding::Buf);

impl CustomData {
    /// Gets an empty value.
    #[cfg(test)]
    pub const EMPTY: Self = Self(encoding::Buf::new_const());

    /// Converts this instance to a component custom ID.
    #[must_use]
    pub fn to_custom_id(&self) -> String {
        encoding::encode_custom_id(&self.0)
    }

    /// Converts this instance to [`ButtonArgs`].
    #[cfg(test)]
    pub fn to_button_args(&self) -> Result<ButtonArgs> {
        encoding::read_button_args(&self.0)
    }

    /// Creates an instance from [`ButtonArgs`].
    #[must_use]
    pub fn from_button_args(args: ButtonArgsRef<'_>) -> Self {
        let mut buf = encoding::Buf::new();
        encoding::write_button_args(&mut buf, args);
        Self(buf)
    }
}
