use crate::buttons::*;

utils::define_simple_error!(Unused(()): "this button is not intended to be used");

/// A sentinel value that can be used to create unique non-overlapping custom IDs.
///
/// Its [`ButtonArgsReply`] implementation will always return an error.
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

impl ButtonArgsReply for None {
    async fn reply(self, _ctx: ButtonContext<'_>) -> HResult {
        Err(Unused(()))?
    }
}
