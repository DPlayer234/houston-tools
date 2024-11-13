use std::borrow::Cow;
use std::mem::swap;
use std::ptr;

use serenity::prelude::*;
use smallvec::SmallVec;

use utils::fields::FieldMut;

use crate::prelude::*;
use crate::modules::{azur, core as core_mod, perks, starboard};

#[cfg(test)]
mod test;

pub mod prelude {
    pub use crate::prelude::*;
    #[allow(unused_imports)]
    pub use super::{ButtonArgs, ButtonArgsRef, ButtonArgsReply, ButtonContext, ButtonMessage, ButtonMessageMode, CustomData, ToCustomData};
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
                        ButtonArgs::$name(v) => ButtonArgsRef::$name(v),
                    )*
                }
            }

            async fn reply(self, ctx: ButtonContext<'_>) -> HResult {
                match self {
                    $(
                        ButtonArgs::$name(args) => args.reply(ctx).await,
                    )*
                }
            }
        }
    };
}

define_button_args! {
    /// Unused button. A sentinel value is used to avoid duplicating custom IDs.
    None(core_mod::buttons::None),
    /// Open the ship detail view.
    ViewShip(azur::buttons::ship::View),
    /// Open the augment detail view.
    ViewAugment(azur::buttons::augment::View),
    /// Open the skill detail view.
    ViewSkill(azur::buttons::skill::View),
    /// Open the ship lines detail view.
    ViewLines(azur::buttons::lines::View),
    /// Open the ship filter list view.
    ViewSearchShip(azur::buttons::search_ship::View),
    /// Open the ship shadow equip details.
    ViewShadowEquip(azur::buttons::shadow_equip::View),
    /// Open the equipment details.
    ViewEquip(azur::buttons::equip::View),
    /// Open the equipment search.
    ViewSearchEquip(azur::buttons::search_equip::View),
    /// Open the augment search.
    ViewSearchAugment(azur::buttons::search_augment::View),
    /// Open the perk store.
    PerksStore(perks::buttons::shop::View),
    /// Open the starboard top view.
    StarboardTop(starboard::buttons::top::View),
    /// Open the starboard top posts view.
    StarboardTopPosts(starboard::buttons::top_posts::View),
}

impl ButtonArgs {
    /// Constructs button arguments from a component custom ID.
    pub fn from_custom_id(id: &str) -> anyhow::Result<Self> {
        let mut bytes = SmallVec::new();
        utils::str_as_data::decode_b65536(&mut bytes, id)?;
        CustomData(bytes).to_button_args()
    }
}

impl<'a> From<&'a ButtonArgs> for ButtonArgsRef<'a> {
    fn from(value: &'a ButtonArgs) -> Self {
        value.borrow()
    }
}

/// Event handler for custom button menus.
pub mod handler {
    use super::*;

    /// To be called in [`EventHandler::interaction_create`].
    pub async fn interaction_create(ctx: Context, interaction: Interaction) {
        // We only care about component interactions.
        let Interaction::Component(interaction) = interaction else { return };

        // Dispatch, then handle errors.
        if let Err(err) = interaction_dispatch(&ctx, &interaction).await {
            handle_dispatch_error(ctx, interaction, err).await
        }
    }

    /// Handles the component interaction dispatch.
    async fn interaction_dispatch(ctx: &Context, interaction: &ComponentInteraction) -> HResult {
        use ComponentInteractionDataKind as Kind;

        let custom_id: &str = match &interaction.data.kind {
            Kind::StringSelect { values } if values.len() == 1 => &values[0],
            Kind::Button => &interaction.data.custom_id,
            _ => anyhow::bail!("Invalid interaction."),
        };

        let args = ButtonArgs::from_custom_id(custom_id)?;
        log::trace!("{}: {:?}", interaction.user.name, args);

        args.reply(ButtonContext {
            serenity: ctx,
            interaction,
            http: ctx.http(),
            data: ctx.data_ref::<HFrameworkData>(),
        }).await
    }

    #[cold]
    async fn handle_dispatch_error(ctx: Context, interaction: ComponentInteraction, err: anyhow::Error) {
        if let Some(ser_err) = err.downcast_ref::<serenity::Error>() {
            // print both errors to preserve the stack trace, if present
            log::warn!("Discord interaction error: {ser_err:?} / {err:?}");
            return;
        }

        log::warn!("Component error: {err:?}");

        let err_text = format!("Button error: ```{err}```");
        let embed = CreateEmbed::new()
            .description(err_text)
            .color(ERROR_EMBED_COLOR);

        let reply = CreateReply::new()
            .ephemeral(true)
            .embed(embed);

        let response = reply.to_slash_followup_response(Default::default());

        let res = interaction.create_followup(ctx.http(), response).await;
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
    fn to_custom_id(&self) -> String {
        self.to_custom_data().to_custom_id()
    }

    /// Converts this instance to custom data.
    #[must_use]
    fn to_custom_data(&self) -> CustomData;

    /// Creates a new button that would switch to a state where one field is changed.
    ///
    /// If the field value is the same, instead returns a disabled button with the sentinel value.
    fn new_button<'a, T: PartialEq>(&mut self, field: impl FieldMut<Self, T>, value: T, sentinel: impl FnOnce(T) -> u16) -> CreateButton<'a> {
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

    /// Creates a new select option that would switch to a state where one field is changed.
    fn new_select_option<'a, T: PartialEq>(&mut self, label: impl Into<Cow<'a, str>>, field: impl FieldMut<Self, T>, value: T) -> CreateSelectMenuOption<'a> {
        let default = *field.get(self) == value;
        let custom_id = self.to_custom_id_with(field, value);

        CreateSelectMenuOption::new(label, custom_id)
            .default_selection(default)
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
    fn to_custom_data(&self) -> CustomData {
        CustomData::from_button_args(self.into())
    }
}

/// Execution context for [`ButtonArgsReply`].
#[derive(Debug, Clone)]
pub struct ButtonContext<'a> {
    /// The serenity context.
    pub serenity: &'a Context,
    /// The source interaction.
    pub interaction: &'a ComponentInteraction,
    /// The HTTP API that may be used.
    pub http: &'a serenity::http::Http,
    /// The bot data.
    pub data: &'a HBotData,
}

impl ButtonContext<'_> {
    /// Replies to the interaction.
    pub async fn reply(&self, create: CreateInteractionResponse<'_>) -> HResult {
        Ok(self.interaction.create_response(self.http, create).await?)
    }

    /// Edits a previous reply to the interaction.
    pub async fn edit_reply(&self, create: EditInteractionResponse<'_>) -> HResult {
        self.interaction.edit_response(self.http, create).await?;
        Ok(())
    }
}

/// Provides a way for button arguments to reply to the interaction.
pub trait ButtonArgsReply: Sized {
    /// Replies to the interaction.
    async fn reply(self, ctx: ButtonContext<'_>) -> HResult;
}

/// Provides a way for button arguments to modify the create-reply payload.
pub trait ButtonMessage: Sized {
    /// Modifies the create-reply payload.
    fn create_reply(self, ctx: ButtonContext<'_>) -> anyhow::Result<CreateReply<'_>>;

    /// How to post the message. Defaults to [`ButtonMessageMode::Edit`].
    #[must_use]
    fn message_mode(&self) -> ButtonMessageMode { ButtonMessageMode::Edit }
}

/// The mode a [`ButtonMessage`] uses to post its message.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ButtonMessageMode {
    /// Edit the existing message of the button.
    #[default] Edit,
    /// Create a new message.
    New,
}

impl<T: ButtonMessage> ButtonArgsReply for T {
    async fn reply(self, ctx: ButtonContext<'_>) -> HResult {
        let mode = self.message_mode();
        let reply = self.create_reply(ctx.clone())?;
        let reply = reply.to_slash_initial_response(Default::default());

        let reply = match mode {
            ButtonMessageMode::New => CreateInteractionResponse::Message(reply),
            ButtonMessageMode::Edit => CreateInteractionResponse::UpdateMessage(reply),
        };

        ctx.reply(reply).await
    }
}

/// Represents custom data for another menu.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CustomData(SmallVec<[u8; 16]>);

impl CustomData {
    /// Gets an empty value.
    pub const EMPTY: Self = Self(SmallVec::new_const());

    /// Converts this instance to a component custom ID.
    #[must_use]
    pub fn to_custom_id(&self) -> String {
        utils::str_as_data::to_b65536(&self.0)
    }

    /// Converts this instance to [`ButtonArgs`].
    pub fn to_button_args(&self) -> anyhow::Result<ButtonArgs> {
        Ok(serde_bare::from_slice(&self.0)?)
    }

    /// Creates an instance from [`ButtonArgs`].
    #[must_use]
    pub fn from_button_args(args: ButtonArgsRef<'_>) -> Self {
        let mut data = SmallVec::new();
        match serde_bare::to_writer(&mut data, &args) {
            Ok(()) => Self(data),
            Err(err) => {
                log::error!("Error [{err:?}] serializing: {args:?}");
                Self::EMPTY
            }
        }
    }
}
