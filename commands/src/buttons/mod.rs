use std::sync::Arc;
use serenity::prelude::*;
use crate::prelude::*;

pub mod azur;

#[derive(Debug, Clone)]
pub struct ButtonEventHandler {
    bot_data: Arc<HBotData>
}

impl ButtonEventHandler {
    pub fn new(bot_data: Arc<HBotData>) -> Self {
        ButtonEventHandler {
            bot_data
        }
    }

    async fn interaction_dispatch(&self, ctx: Context, interaction: ComponentInteraction) -> HResult {
        match ButtonArgs::from_custom_id(&interaction.data.custom_id)? {
            ButtonArgs::None(_) => Ok(()),
            ButtonArgs::ViewShip(view_ship) => self.inner_dispatch(ctx, interaction, view_ship).await,
            ButtonArgs::ViewAugment(view_augment) => self.inner_dispatch(ctx, interaction, view_augment).await,
            ButtonArgs::ViewSkill(view_skill) => self.inner_dispatch(ctx, interaction, view_skill).await,
        }
    }

    async fn inner_dispatch<T: ButtonArgsModify>(&self, ctx: Context, interaction: ComponentInteraction, args: T) -> HResult {
        let user_data = self.bot_data.get_user_data(interaction.user.id);
        let reply = args.modify(&self.bot_data, user_data.create_reply())?;
        let response_message = reply.to_slash_initial_response(Default::default());
        interaction.create_response(ctx, CreateInteractionResponse::UpdateMessage(response_message)).await?;
        Ok(())
    }
}

#[serenity::async_trait]
impl serenity::client::EventHandler for ButtonEventHandler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Component(interaction) = interaction {
            if let Err(err) = self.interaction_dispatch(ctx, interaction).await {
                println!("Dispatch error: {}", err);
            }
        }
    }
}

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub enum ButtonArgs {
    None(Sentinel),
    ViewShip(azur::ship::ViewShip),
    ViewAugment(azur::augment::ViewAugment),
    ViewSkill(azur::skill::ViewSkill),
}

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct Sentinel {
    pub key: u32,
    pub value: u32
}

pub trait ToButtonArgsId {
    fn to_custom_id(self) -> String;

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
}

pub trait ButtonArgsModify {
    fn modify(self, data: &HBotData, create: CreateReply) -> anyhow::Result<CreateReply>;
}

impl ButtonArgs {
    pub fn from_custom_id(id: &str) -> anyhow::Result<ButtonArgs> {
        let bytes = from_base256_string(id)?;
        let args = bitcode::decode(&bytes)?;
        Ok(args)
    }
}

impl<T: Into<ButtonArgs>> ToButtonArgsId for T {
    fn to_custom_id(self) -> String {
        let args: ButtonArgs = self.into();
        let encoded = bitcode::encode(&args);
        to_base256_string(&encoded)
    }
}

impl Sentinel {
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

#[macro_export]
macro_rules! new_button {
    ($var:expr => $field:ident = $value:expr; $sentinel:expr) => {{
        let mut new_state = $var.clone();
        new_state.$field = $value.clone();

        let disabled = $var.$field == new_state.$field;
        if disabled {
            CreateButton::new($crate::buttons::ToButtonArgsId::to_custom_id($crate::buttons::ButtonArgs::None($sentinel))).disabled(true)
        } else {
            CreateButton::new($crate::buttons::ToButtonArgsId::to_custom_id(new_state))
        }
    }};
}
