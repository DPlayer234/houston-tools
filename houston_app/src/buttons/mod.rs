use std::sync::Arc;

use serenity::prelude::*;
use utils::fields::FieldMut;

pub use crate::prelude::*;

pub mod azur;

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
            /// Unused button. A sentinel value is used to avoid duplicating custom IDs.
            None(Sentinel),
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
            /// Unused button. A sentinel value is used to avoid duplicating custom IDs.
            None(&'a Sentinel),
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

        impl<'a> From<&'a ButtonArgs> for ButtonArgsRef<'a> {
            fn from(value: &'a ButtonArgs) -> Self {
                value.borrow()
            }
        }

        impl ButtonArgs {
            /// Borrows the inner data.
            pub const fn borrow(&self) -> ButtonArgsRef {
                match self {
                    ButtonArgs::None(v) => ButtonArgsRef::None(v),
                    $(
                        ButtonArgs::$name(v) => ButtonArgsRef::$name(v),
                    )*
                }
            }
        }

        impl ButtonArgsModify for ButtonArgs {
            fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
                match self {
                    ButtonArgs::None(_) => Ok(create),
                    $(
                        ButtonArgs::$name(args) => args.modify(data, create),
                    )*
                }
            }
        }

        impl ButtonEventHandler {
            async fn interaction_dispatch_dyn(&self, ctx: &Context, interaction: &ComponentInteraction, args: ButtonArgs) -> HResult {
                match args {
                    ButtonArgs::None(_) => Ok(()),
                    $(
                        ButtonArgs::$name(args) => self.interaction_dispatch_to(ctx, interaction, args).await,
                    )*
                }
            }
        }
    };
}

define_button_args! {
    /// Creates a new message.
    AsNewMessage(AsNewMessage),
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

/// Event handler for custom button menus.
#[derive(Debug, Clone)]
pub struct ButtonEventHandler {
    bot_data: Arc<HBotData>
}

impl ButtonEventHandler {
    /// Creates a new handler.
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
        self.interaction_dispatch_dyn(ctx, interaction, args).await
    }

    /// Dispatches the component interaction to specified arguments.
    async fn interaction_dispatch_to<T: ButtonArgsModify>(&self, ctx: &Context, interaction: &ComponentInteraction, args: T) -> HResult {
        let user_data = self.bot_data.get_user_data(interaction.user.id);
        let reply = args.modify(&self.bot_data, user_data.create_reply())?;
        let reply = T::make_interaction_response(reply);
        interaction.create_response(ctx, reply).await?;
        Ok(())
    }

    #[cold]
    async fn handle_dispatch_error(&self, ctx: Context, interaction: ComponentInteraction, err: anyhow::Error) {
        if let Some(err) = err.downcast_ref::<serenity::Error>() {
            println!("Discord interaction error: {}", err);
        } else {
            println!("Component error: {}", err);

            let err_text = format!("Button error: ```{}```", err);
            let reply = CreateReply::default().ephemeral(true)
                .embed(CreateEmbed::new().description(err_text).color(ERROR_EMBED_COLOR));
            let response = reply.to_slash_initial_response(Default::default());

            let res = interaction.create_response(ctx, CreateInteractionResponse::Message(response)).await;
            if let Err(res) = res {
                println!("Error sending component error: {}", res);
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

/// A sentinel value that can be used to create unique non-overlapping custom IDs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Sentinel {
    pub key: u16,
    pub value: u16
}

/// Wraps another [`ButtonArgs`] value and makes it
/// send a new message rather than using its default behavior.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AsNewMessage(CustomData);

/// Represents custom data for another menu.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomData(Vec<u8>);

/// Provides a way to convert an object into a component custom ID.
///
/// This is auto-implemented for every type held by [`ButtonArgs`].
pub trait ToButtonArgsId: Sized {
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
    fn new_button<T: PartialEq>(&mut self, field: impl FieldMut<Self, T>, value: T, sentinel: impl FnOnce(T) -> u16) -> CreateButton {
        let disabled = *field.get(self) == value;
        if disabled {
            let sentinel = Sentinel::new(field_sentinel_key(self, field), sentinel(value));
            CreateButton::new(ButtonArgs::None(sentinel).to_custom_id()).disabled(true)
        } else {
            let custom_id = self.to_custom_id_with(field, value);
            CreateButton::new(custom_id)
        }
    }

    /// Creates a new select option that would switch to a state where one field is changed.
    fn new_select_option<T: PartialEq>(&mut self, label: impl Into<String>, field: impl FieldMut<Self, T>, value: T) -> CreateSelectMenuOption {
        let default = *field.get(self) == value;
        let custom_id = self.to_custom_id_with(field, value);

        CreateSelectMenuOption::new(label, custom_id)
            .default_selection(default)
    }

    /// Creates a custom ID with one field replaced.
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

fn field_sentinel_key<S, T>(obj: &S, field: impl FieldMut<S, T>) -> u16 {
    // The value returned here is intended to be unique for a given object.
    // It isn't used in any way other than as a discriminator.
    field.get(obj) as *const T as u16
}

/// Provides a way for button arguments to modify the create-reply payload.
pub trait ButtonArgsModify: Sized {
    /// Modifies the create-reply payload.
    #[must_use]
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply>;

    fn make_interaction_response(create: CreateReply) -> CreateInteractionResponse {
        let edit = create.to_slash_initial_response(Default::default());
        CreateInteractionResponse::UpdateMessage(edit)
    }
}

impl ButtonArgs {
    /// Constructs button arguments from a component custom ID.
    #[must_use]
    pub fn from_custom_id(id: &str) -> anyhow::Result<ButtonArgs> {
        let bytes = utils::str_as_data::from_b65536(id)?;
        CustomData(bytes).to_button_args()
    }
}

impl<T> ToButtonArgsId for T
where for<'a> &'a T: Into<ButtonArgsRef<'a>> {
    fn to_custom_data(&self) -> CustomData {
        CustomData::from_button_args(self)
    }
}

impl Sentinel {
    /// Create a new sentinel value.
    pub const fn new(key: u16, value: u16) -> Self {
        Self { key, value }
    }
}

impl AsNewMessage {
    pub fn new<'a>(value: impl Into<ButtonArgsRef<'a>>) -> Self {
        Self(CustomData::from_button_args(value))
    }
}

impl ButtonArgsModify for AsNewMessage {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply> {
        let args = self.0.to_button_args()?;
        args.modify(data, create)
    }

    fn make_interaction_response(create: CreateReply) -> CreateInteractionResponse {
        let edit = create.to_slash_initial_response(Default::default());
        CreateInteractionResponse::Message(edit)
    }
}

impl CustomData {
    /// Converts this instance to a component custom ID.
    #[must_use]
    pub fn to_custom_id(&self) -> String {
        utils::str_as_data::to_b65536(&self.0)
    }

    /// Converts this instance to [`ButtonArgs`].
    #[must_use]
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
                println!("Error [{err:?}] serializing: {args:?}");
                Self(Vec::new())
            }
        }
    }
}
