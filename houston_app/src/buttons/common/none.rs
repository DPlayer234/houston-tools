use crate::buttons::*;

utils::define_simple_error!(Unused: "this button is not intended to be used");

/// A sentinel value that can be used to create unique non-overlapping custom IDs.
///
/// Its [`ButtonArgsModify`] implementation will always return an error.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct None {
    key: u16,
    value: u16
}

impl None {
    /// Create a new sentinel value.
    pub const fn new(key: u16, value: u16) -> Self {
        Self {
            key,
            value
        }
    }
}

impl ButtonArgsModify for None {
    fn modify(self, _data: &HBotData, _create: CreateReply) -> anyhow::Result<CreateReply> {
        Err(Unused)?
    }

    fn make_interaction_response(_create: CreateReply) -> CreateInteractionResponse {
        CreateInteractionResponse::Acknowledge
    }
}
