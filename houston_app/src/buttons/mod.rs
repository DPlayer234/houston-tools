use std::sync::Arc;

use serenity::prelude::*;
use utils::fields::FieldMut;

pub use crate::prelude::*;

pub mod azur;

utils::define_simple_error!(InvalidInteractionError: "Invalid interaction.");

macro_rules! define_button_args {
    ($($(#[$attr:meta])* $name:ident($Ty:ty)),* $(,)?) => {
        /// The supported button interaction arguments.
        #[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
        pub enum ButtonArgs {
            /// Unused button. A sentinel value is used to avoid duplicating custom IDs.
            None(Sentinel),
            $(
                $(#[$attr])*
                $name($Ty),
            )*
        }

        $(
            impl From<$Ty> for ButtonArgs {
                fn from(value: $Ty) -> Self {
                    Self::$name(value)
                }
            }
        )*

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
    pub fn new(bot_data: Arc<HBotData>) -> Self {
        ButtonEventHandler {
            bot_data
        }
    }

    /// Handles the component interaction dispatch.
    async fn interaction_dispatch(&self, ctx: &Context, interaction: &ComponentInteraction) -> HResult {
        let args = match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => {
                ButtonArgs::from_custom_id(&values.iter().next().ok_or(InvalidInteractionError)?)?
            }
            _ => {
                ButtonArgs::from_custom_id(&interaction.data.custom_id)?
            }
        };

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
#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct Sentinel {
    pub key: u32,
    pub value: u32
}

/// Wraps another [`ButtonArgs`] value and makes it
/// send a new message rather than using its default behavior.
#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct AsNewMessage(CustomData);

/// Represents custom data for another menu.
#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct CustomData(Vec<u8>);

/// Provides a way to convert an object into a component custom ID.
///
/// This is auto-implemented for all [`Into<ButtonArgs>`].
pub trait ToButtonArgsId: Sized {
    /// Converts this instance to a component custom ID.
    #[must_use]
    fn into_custom_id(self) -> String {
        self.into_custom_data().to_custom_id()
    }

    /// Converts this instance to custom data.
    #[must_use]
    fn into_custom_data(self) -> CustomData;

    /// Creates a new button that would switch to a state where one field is changed.
    ///
    /// If the field value is the same, instead returns a disabled button with the sentinel value.
    fn new_button<T: PartialEq>(&self, field: impl FieldMut<Self, T>, value: T, sentinel: impl FnOnce() -> Sentinel) -> CreateButton
    where Self: Clone {
        let disabled = *field.get(&self) == value;
        if disabled {
            CreateButton::new(ButtonArgs::None(sentinel()).into_custom_id()).disabled(true)
        } else {
            let mut new_state = self.clone();
            *field.get_mut(&mut new_state) = value;

            CreateButton::new(new_state.into_custom_id())
        }
    }

    /// Creates a new select option that would switch to a state where one field is changed.
    fn new_select_option<T: PartialEq>(&self, label: impl Into<String>, field: impl FieldMut<Self, T>, value: T) -> CreateSelectMenuOption
    where Self: Clone {
        let mut new_state = self.clone();
        *field.get_mut(&mut new_state) = value;

        let default = field.get(&self) == field.get(&new_state);
        CreateSelectMenuOption::new(label, new_state.into_custom_id())
            .default_selection(default)
    }
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

impl<T: Into<ButtonArgs>> ToButtonArgsId for T {
    #[must_use]
    fn into_custom_data(self) -> CustomData {
        let args: ButtonArgs = self.into();
        CustomData::from_button_args(&args)
    }
}

impl Sentinel {
    /// Create a new sentinel value.
    pub fn new(key: u32, value: u32) -> Self {
        Self { key, value }
    }
}

impl AsNewMessage {
    pub fn new(value: impl Into<ButtonArgs>) -> Self {
        Self(value.into_custom_data())
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
        Ok(bitcode::decode(&self.0)?)
    }

    /// Creates an instance from [`ButtonArgs`].
    #[must_use]
    pub fn from_button_args(args: &ButtonArgs) -> Self {
        Self(bitcode::encode(args))
    }
}
