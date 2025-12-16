use serenity::model::application::ComponentInteractionDataKind::StringSelect;

use crate::encoding;
use crate::prelude::*;

/// A select menu custom ID that delegates to a custom ID stored within the
/// selected option value.
///
/// [`ButtonAction::key`](crate::ButtonAction::key) = [`u16::MAX - 2`](u16::MAX)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SelectNav(u8);

impl SelectNav {
    /// Create a new value.
    ///
    /// The index needs to be unique for message.
    pub const fn new(index: u8) -> Self {
        Self(index)
    }
}

button_value!(SelectNav, (u16::MAX - 2) as usize);
impl ButtonReply for SelectNav {
    async fn reply(self, ctx: ButtonContext<'_>) -> crate::Result {
        if let StringSelect { values } = &ctx.interaction.data.kind
            && let [custom_id] = values.as_slice()
        {
            let mut buf = encoding::StackBuf::new();
            let mut decoder = encoding::decode_custom_id(&mut buf, custom_id)?;
            let key = decoder.read_key()?;
            let action = ctx.inner.state.action(key)?;

            (action.invoke_button)(ctx, decoder).await
        } else {
            anyhow::bail!("invalid SelectNav target component")
        }
    }
}
