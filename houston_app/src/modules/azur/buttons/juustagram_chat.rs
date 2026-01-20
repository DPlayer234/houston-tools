use arrayvec::ArrayVec;
use azur_lane::juustagram::*;
use utils::text::{WriteStr as _, truncate};

use super::AzurParseError;
use crate::buttons::prelude::*;
use crate::config::emoji;
use crate::fmt::discord::escape_markdown;
use crate::modules::azur::{GameData, LazyData};

#[derive(Debug, Clone, Serialize, Deserialize, ConstBuilder)]
pub struct View<'v> {
    chat_id: u32,
    #[serde(borrow)]
    #[builder(default = &[0])]
    flags: &'v [u8],
    #[serde(borrow)]
    #[builder(default = None, setter(strip_option))]
    back: Option<Nav<'v>>,
}

impl View<'_> {
    /// Modifies the create-reply with preresolved ship data.
    pub fn create_with_chat<'a>(
        mut self,
        data: &'a HBotData,
        azur: &'a LazyData,
        chat: &'a Chat,
    ) -> Result<CreateReply<'a>> {
        fn get_sender_name(azur: &GameData, sender_id: u32) -> &str {
            if sender_id == 0 {
                return "<You>";
            }

            azur.ship_by_id(sender_id).map_or("<unknown>", |s| &s.name)
        }

        let mut content = String::new();
        let mut selection = None;

        for entry in &chat.entries {
            // if the chat entry does not have the right flag, we skip it
            if !self.flags.contains(&entry.flag) {
                continue;
            }

            // print the content of the chat entry
            match &entry.content {
                ChatContent::Message { sender_id, text } => writeln!(
                    content,
                    "- **{}:** {}",
                    get_sender_name(azur.game_data(), *sender_id),
                    escape_markdown(text)
                ),
                ChatContent::Sticker { sender_id, label } => writeln!(
                    content,
                    "- **{}:** {}",
                    get_sender_name(azur.game_data(), *sender_id),
                    label
                ),
                ChatContent::System { text } => writeln!(content, "- [{text}]"),
            }

            // if there are options, we stop if we hold the flag for neither of them
            if let Some(options) = &entry.options
                && options.iter().all(|o| !self.flags.contains(&o.flag))
            {
                selection = Some(options);
                break;
            }
        }

        let mut components = ComponentVec::new();
        components.push(CreateTextDisplay::new(format!("### {}", chat.name)));
        components.push(CreateSeparator::new(true));

        // this may be janky, possibly rework the limit
        components.push(CreateTextDisplay::new(truncate(content, 3800)));

        if let Some(options) = selection {
            components.push(CreateSeparator::new(true));

            for option in options {
                let mut flags = <ArrayVec<u8, 64>>::new();
                flags.try_extend_from_slice(self.flags)?;
                flags.try_push(option.flag)?;
                let flags = flags.as_slice();

                let new_view = View {
                    flags,
                    back: self.back.clone(),
                    ..self
                };

                let button = CreateButton::new(new_view.to_custom_id())
                    .label(truncate(&option.value, 80))
                    .style(ButtonStyle::Secondary);

                components.push(CreateActionRow::buttons(vec![button]));
            }
        }

        let mut nav_row = Vec::new();
        if let Some(back) = &self.back {
            nav_row.push(
                CreateButton::new(back.to_custom_id())
                    .emoji(emoji::back())
                    .label("Back"),
            );
        }

        if let Some((_, new_flags)) = self.flags.split_last() {
            let button = self
                .new_button(|s| &mut s.flags, new_flags, |_| u16::MAX)
                .label("Undo");

            nav_row.push(button);
        }

        if !nav_row.is_empty() {
            components.push(CreateSeparator::new(true));
            components.push(CreateActionRow::buttons(nav_row));
        }

        Ok(CreateReply::new().components_v2(components![
            CreateContainer::new(components).accent_color(data.config().embed_color)
        ]))
    }
}

button_value!(for<'v> View<'v>, 16);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        let data = ctx.data_ref();
        let azur = data.config().azur()?;
        let chat = azur
            .game_data()
            .juustagram_chat_by_id(self.chat_id)
            .ok_or(AzurParseError::JuustagramChat)?;

        let create = self.create_with_chat(data, azur, chat)?;
        ctx.edit(create.into()).await?;
        Ok(())
    }
}
