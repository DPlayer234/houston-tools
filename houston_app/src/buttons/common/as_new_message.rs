use crate::buttons::*;

/// Wraps another [`ButtonArgs`] value and makes it
/// send a new message rather than using its default behavior.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AsNewMessage(CustomData);

impl AsNewMessage {
    /// Create a new [`AsNewMessage`] value with the specified menu.
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
