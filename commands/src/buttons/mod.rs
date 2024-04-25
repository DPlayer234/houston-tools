use std::sync::Arc;
use bitcode::*;
use serenity::prelude::*;
use crate::internal::prelude::*;

#[derive(Debug, Clone)]
pub struct ButtonEventHandler {
    _bot_data: Arc<HBotData>
}

impl ButtonEventHandler {
    pub fn new(bot_data: Arc<HBotData>) -> Self {
        ButtonEventHandler {
            _bot_data: bot_data
        }
    }
}

#[serenity::async_trait]
impl serenity::client::EventHandler for ButtonEventHandler {
    async fn interaction_create(&self, _ctx: Context, interaction: Interaction) {
        if let Interaction::Component(_interaction) = interaction {
            todo!()
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum ButtonArgs {

}
