use std::sync::Arc;

use serenity::prelude::*;
use utils::fields::FieldMut;

pub use crate::prelude::*;

pub mod azur;
pub mod common;

utils::define_simple_error!(InvalidInteractionError: "Invalid interaction.");

/// Helper macro that repeats needed code for every [`ButtonArgs`] variant.
macro_rules! define_button_args {
    ($($(#[$attr:meta])* $name:ident($Ty:ty)),* $(,)?) => {
        /// The supported button interaction arguments.
        ///
        /// This is owned data that can be deserialized into.
        /// To serialize it, call [`ButtonArgs::borrow`] first.
        #[derive(Debug, Clone, serde::Deserialize)]
        pub enum ButtonArgs {
            $(
                $(#[$attr])*
                $name($Ty),
            )*
        }

        /// The supported button interaction arguments.
        ///
        /// This is borrowed data that can be serialized.
        #[derive(Debug, Clone, Copy, serde::Serialize)]
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
            pub const fn borrow(&self) -> ButtonArgsRef {
                match self {
                    $(
                        ButtonArgs::$name(v) => ButtonArgsRef::$name(v),
                    )*
                }
            }

            pub async fn reply(self, ctx: ButtonContext<'_>) -> HResult {
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
    None(common::None),
    /// Open the ship detail view.
    ViewShip(azur::ship::View),
    /// Open the augment detail view.
    ViewAugment(azur::augment::View),
    /// Open the skill detail view.
    ViewSkill(azur::skill::View),
    /// Open the ship lines detail view.
    ViewLines(azur::lines::View),
    /// Open the ship filter list view.
    ViewSearchShip(azur::search_ship::View),
    /// Open the ship shadow equip details.
    ViewShadowEquip(azur::shadow_equip::View),
    /// Open the equipment details.
    ViewEquip(azur::equip::View),
    /// Open the equipment search.
    ViewSearchEquip(azur::search_equip::View),
}

impl ButtonArgs {
    /// Constructs button arguments from a component custom ID.
    pub fn from_custom_id(id: &str) -> anyhow::Result<ButtonArgs> {
        let bytes = utils::str_as_data::from_b65536(id)?;
        CustomData(bytes).to_button_args()
    }
}

impl<'a> From<&'a ButtonArgs> for ButtonArgsRef<'a> {
    fn from(value: &'a ButtonArgs) -> Self {
        value.borrow()
    }
}

/// Event handler for custom button menus.
#[derive(Debug, Clone)]
pub struct ButtonEventHandler {
    bot_data: Arc<HBotData>
}

impl ButtonEventHandler {
    /// Creates a new handler.
    #[must_use]
    pub const fn new(bot_data: Arc<HBotData>) -> Self {
        ButtonEventHandler {
            bot_data
        }
    }

    /// Handles the component interaction dispatch.
    async fn interaction_dispatch(&self, ctx: &Context, interaction: &ComponentInteraction) -> HResult {
        use ComponentInteractionDataKind as Kind;

        let custom_id = match &interaction.data.kind {
            Kind::StringSelect { values } if values.len() == 1 => &values[0],
            Kind::Button => &interaction.data.custom_id,
            _ => Err(InvalidInteractionError)?,
        };

        let args = ButtonArgs::from_custom_id(custom_id)?;
        log::trace!("{}: {:?}", interaction.user.name, args);

        args.reply(ButtonContext {
            interaction,
            http: &ctx.http,
            data: &self.bot_data
        }).await
    }

    #[cold]
    async fn handle_dispatch_error(&self, ctx: Context, interaction: ComponentInteraction, err: anyhow::Error) {
        if let Some(err) = err.downcast_ref::<serenity::Error>() {
            log::warn!("Discord interaction error: {err}");
        } else {
            log::warn!("Component error: {err:?}");

            let err_text = format!("Button error: ```{err}```");
            let reply = CreateReply::default().ephemeral(true)
                .embed(CreateEmbed::new().description(err_text).color(ERROR_EMBED_COLOR));
            let response = reply.to_slash_initial_response(Default::default());

            let res = interaction.create_response(ctx, CreateInteractionResponse::Message(response)).await;
            if let Err(res) = res {
                log::warn!("Error sending component error: {res}");
            }
        }
    }
}

#[serenity::async_trait]
impl serenity::client::EventHandler for ButtonEventHandler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        // We only care about component interactions.
        let Interaction::Component(interaction) = interaction else { return };

        // Dispatch, then handle errors.
        if let Err(err) = self.interaction_dispatch(&ctx, &interaction).await {
            self.handle_dispatch_error(ctx, interaction, err).await
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
    #[must_use]
    fn new_button<T: PartialEq>(&mut self, field: impl FieldMut<Self, T>, value: T, sentinel: impl FnOnce(T) -> u16) -> CreateButton {
        let disabled = *field.get(self) == value;
        if disabled {
            // This value is intended to be unique for a given object.
            // It isn't used in any way other than as a discriminator.
            let sentinel_key = field.get(self) as *const T as u16;

            let sentinel = common::None::new(sentinel_key, sentinel(value));
            CreateButton::new(ButtonArgs::None(sentinel).to_custom_id()).disabled(true)
        } else {
            let custom_id = self.to_custom_id_with(field, value);
            CreateButton::new(custom_id)
        }
    }

    /// Creates a new select option that would switch to a state where one field is changed.
    #[must_use]
    fn new_select_option<T: PartialEq>(&mut self, label: impl Into<String>, field: impl FieldMut<Self, T>, value: T) -> CreateSelectMenuOption {
        let default = *field.get(self) == value;
        let custom_id = self.to_custom_id_with(field, value);

        CreateSelectMenuOption::new(label, custom_id)
            .default_selection(default)
    }

    /// Creates a custom ID with one field replaced.
    #[must_use]
    fn to_custom_id_with<T>(&mut self, field: impl FieldMut<Self, T>, mut value: T) -> String {
        // Swap new value into the field
        std::mem::swap(field.get_mut(self), &mut value);
        // Create the custom ID
        let custom_id = self.to_custom_id();
        // Move original value back into field, dropping the new value.
        *field.get_mut(self) = value;

        custom_id
    }
}

impl<T> ToCustomData for T
where for<'a> &'a T: Into<ButtonArgsRef<'a>> {
    fn to_custom_data(&self) -> CustomData {
        CustomData::from_button_args(self)
    }
}

/// Execution context for [`ButtonArgsReply`].
#[derive(Debug, Clone)]
pub struct ButtonContext<'a> {
    pub interaction: &'a ComponentInteraction,
    pub http: &'a serenity::all::Http,
    pub data: &'a HBotData,
}

impl ButtonContext<'_> {
    /// Replies to the interaction.
    pub async fn reply(&self, create: CreateInteractionResponse) -> HResult {
        Ok(self.interaction.create_response(self.http, create).await?)
    }

    /// Creates a fitting base reply.
    pub fn create_reply(&self) -> CreateReply {
        self.data.get_user_data(self.interaction.user.id).create_reply()
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
    fn create_reply(self, ctx: ButtonContext<'_>) -> anyhow::Result<CreateReply>;

    /// How to post the message. Defaults to [`ButtonMessageMode::Edit`]
    fn message_mode(&self) -> ButtonMessageMode { ButtonMessageMode::Edit }
}

/// The mode a [`ButtonMessage`] uses to post its message.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ButtonMessageMode {
    #[default] Edit,
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

        Ok(ctx.reply(reply).await?)
    }
}

/// Represents custom data for another menu.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomData(Vec<u8>);

impl CustomData {
    /// Gets an empty value.
    pub const EMPTY: Self = Self(Vec::new());

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
    pub fn from_button_args<'a>(args: impl Into<ButtonArgsRef<'a>>) -> Self {
        let args: ButtonArgsRef = args.into();
        match serde_bare::to_vec(&args) {
            Ok(data) => Self(data),
            Err(err) => {
                log::error!("Error [{err:?}] serializing: {args:?}");
                Self::EMPTY
            }
        }
    }
}
