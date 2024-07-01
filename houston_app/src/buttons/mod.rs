use std::sync::Arc;

use serenity::prelude::*;

pub use crate::prelude::*;

pub mod azur;

utils::define_simple_error!(InvalidInteractionError: "Invalid interaction.");

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

        match args {
            ButtonArgs::None(_) => Ok(()),
            ButtonArgs::ViewShip(view_ship) => self.interaction_dispatch_to(ctx, interaction, view_ship).await,
            ButtonArgs::ViewAugment(view_augment) => self.interaction_dispatch_to(ctx, interaction, view_augment).await,
            ButtonArgs::ViewSkill(view_skill) => self.interaction_dispatch_to(ctx, interaction, view_skill).await,
            ButtonArgs::ViewLines(view_lines) => self.interaction_dispatch_to(ctx, interaction, view_lines).await,
            ButtonArgs::ViewSearchShip(view_filter) => self.interaction_dispatch_to(ctx, interaction, view_filter).await,
            ButtonArgs::ViewShadowEquip(view_shadow) => self.interaction_dispatch_to(ctx, interaction, view_shadow).await,
            ButtonArgs::ViewEquip(view_equip) => self.interaction_dispatch_to(ctx, interaction, view_equip).await,
        }
    }

    /// Dispatches the component interaction to specified arguments.
    async fn interaction_dispatch_to<T: ButtonArgsModify>(&self, ctx: &Context, interaction: &ComponentInteraction, args: T) -> HResult {
        let user_data = self.bot_data.get_user_data(interaction.user.id);
        let reply = args.modify(&self.bot_data, user_data.create_reply())?;
        let response_message = reply.to_slash_initial_response(Default::default());
        interaction.create_response(ctx, CreateInteractionResponse::UpdateMessage(response_message)).await?;
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

/// The supported button interaction arguments.
#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub enum ButtonArgs {
    /// Unused button. A sentinel value is used to avoid duplicating custom IDs.
    None(Sentinel),
    /// Open the ship detail view.
    ViewShip(azur::ship::ViewShip),
    /// Open the augment detail view.
    ViewAugment(azur::augment::ViewAugment),
    /// Open the skill detail view.
    ViewSkill(azur::skill::ViewSkill),
    /// Open the ship lines detail view.
    ViewLines(azur::lines::ViewLines),
    /// Open the ship filter list view.
    ViewSearchShip(azur::search_ship::ViewSearchShip),
    /// Open the ship shadow equip details.
    ViewShadowEquip(azur::shadow_equip::ViewShadowEquip),
    /// Open the equipment details.
    ViewEquip(azur::equip::ViewEquip),
}

/// A sentinel value that can be used to create unique non-overlapping custom IDs.
#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct Sentinel {
    pub key: u32,
    pub value: u32
}

/// Provides a way to convert an object into a component custom ID.
///
/// This is auto-implemented for all [`Into<ButtonArgs>`].
pub trait ToButtonArgsId {
    /// Converts this instance to a component custom ID.
    fn to_custom_id(self) -> String;

    /// Creates a new button that would switch to a state where one field is changed.
    ///
    /// If the field value is the same, instead returns a disabled button with the sentinel value.
    fn new_button<T: PartialEq>(&self, field: impl utils::Field<Self, T>, value: T, sentinel: impl FnOnce() -> Sentinel) -> CreateButton
    where Self: Clone {
        let mut new_state = self.clone();
        *field.get_mut(&mut new_state) = value;

        let disabled = field.get(&self) == field.get(&new_state);
        if disabled {
            CreateButton::new(ButtonArgs::None(sentinel()).to_custom_id()).disabled(true)
        } else {
            CreateButton::new(new_state.to_custom_id())
        }
    }

    /// Creates a new select option that would switch to a state where one field is changed.
    fn new_select_option<T: PartialEq>(&self, label: impl Into<String>, field: impl utils::Field<Self, T>, value: T) -> CreateSelectMenuOption
    where Self: Clone {
        let mut new_state = self.clone();
        *field.get_mut(&mut new_state) = value;

        let default = field.get(&self) == field.get(&new_state);
        CreateSelectMenuOption::new(label, new_state.to_custom_id())
            .default_selection(default)
    }
}

/// Provides a way for button arguments to modify the create-reply payload.
pub trait ButtonArgsModify: Sized {
    /// Modifies the create-reply payload.
    #[must_use]
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply>;
}

impl ButtonArgs {
    /// Constructs button arguments from a component custom ID.
    #[must_use]
    pub fn from_custom_id(id: &str) -> anyhow::Result<ButtonArgs> {
        let bytes = from_base256_string(id)?;
        let args = bitcode::decode(&bytes)?;
        Ok(args)
    }
}

impl<T: Into<ButtonArgs>> ToButtonArgsId for T {
    #[must_use]
    fn to_custom_id(self) -> String {
        let args: ButtonArgs = self.into();
        let encoded = bitcode::encode(&args);
        to_base256_string(&encoded)
    }
}

impl Sentinel {
    /// Create a new sentinel value.
    pub fn new(key: u32, value: u32) -> Self {
        Self { key, value }
    }
}

fn to_base256_string(bytes: &[u8]) -> String {
    bytes.into_iter().map(|b| char::from(*b)).collect()
}

fn from_base256_string(str: &str) -> Result<Vec<u8>, std::char::TryFromCharError> {
    str.chars().map(|c| u8::try_from(c)).collect()
}
