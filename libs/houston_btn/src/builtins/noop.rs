use crate::prelude::*;

/// A sentinel value that can be used to create unique non-overlapping custom
/// IDs.
///
/// Its [`ButtonReply`] implementation will always return an error.
///
/// [`ButtonAction::key`](crate::ButtonAction::key) = [`u16::MAX`]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Noop {
    key: u16,
    value: u16,
}

impl Noop {
    /// Create a new sentinel value.
    pub const fn new(key: u16, value: u16) -> Self {
        Self { key, value }
    }
}

button_value!(Noop, u16::MAX as usize);
impl ButtonReply for Noop {
    async fn reply(self, _ctx: ButtonContext<'_>) -> crate::Result {
        anyhow::bail!("this button is not intended to be used");
    }
}
