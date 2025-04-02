use crate::buttons::prelude::*;

/// A sentinel value that can be used to create unique non-overlapping custom
/// IDs.
///
/// Its [`ButtonArgsReply`] implementation will always return an error.
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

impl ButtonArgsReply for Noop {
    async fn reply(self, _ctx: ButtonContext<'_>) -> Result {
        anyhow::bail!("this button is not intended to be used");
    }
}
