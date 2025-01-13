use arrayvec::ArrayVec;
use azur_lane::juustagram::*;
use utils::text::truncate;
use utils::text::write_str::*;

use super::AzurParseError;
use crate::buttons::prelude::*;
use crate::fmt::discord::escape_markdown;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    chat_id: u32,
    flags: ArrayVec<u8, 20>,
    back: Option<CustomData>,
}

impl View {
    pub fn new(chat_id: u32) -> Self {
        let mut flags = ArrayVec::new();
        flags.push(0u8);

        Self {
            chat_id,
            flags,
            back: None,
        }
    }

    pub fn back(mut self, back: CustomData) -> Self {
        self.back = Some(back);
        self
    }

    /// Modifies the create-reply with preresolved ship data.
    pub fn create_with_chat<'a>(mut self, data: &'a HBotData, chat: &'a Chat) -> CreateReply<'a> {
        let mut content = String::new();
        let mut components = Vec::new();

        let mut nav_row = Vec::new();
        if let Some(back) = &self.back {
            nav_row.push(
                CreateButton::new(back.to_custom_id())
                    .emoji('⏪')
                    .label("Back"),
            );
        }

        if self.flags.len() > 1 {
            let mut new_flags = self.flags.clone();
            _ = new_flags.pop();

            nav_row.push(
                self.new_button(|s| &mut s.flags, new_flags, |_| u16::MAX)
                    .label("Undo"),
            );
        }

        if !nav_row.is_empty() {
            components.push(CreateActionRow::buttons(nav_row));
        }

        fn get_sender_name(data: &HBotData, sender_id: u32) -> &str {
            if sender_id == 0 {
                return "<You>";
            }

            data.azur_lane()
                .ship_by_id(sender_id)
                .map_or("<unknown>", |s| &s.name)
        }

        for entry in &chat.entries {
            // if the chat entry does not have the right flag, we skip it
            if !self.flags.contains(&entry.flag) {
                continue;
            }

            // print the content of the chat entry
            match &entry.content {
                ChatContent::Message { sender_id, text } => writeln_str!(
                    content,
                    "- **{}:** {}",
                    get_sender_name(data, *sender_id),
                    escape_markdown(text)
                ),
                ChatContent::Sticker { sender_id, label } => writeln_str!(
                    content,
                    "- **{}:** {}",
                    get_sender_name(data, *sender_id),
                    label
                ),
                ChatContent::System { text } => writeln_str!(content, "- [{}]", text),
            }

            // if there are options, we stop if we hold the flag for neither of them
            if let Some(options) = &entry.options {
                if options.iter().all(|o| !self.flags.contains(&o.flag)) {
                    for option in options {
                        let mut new_flags = self.flags.clone();
                        _ = new_flags.try_push(option.flag);

                        let button = self
                            .new_button(|s| &mut s.flags, new_flags, |_| option.flag.into())
                            .label(truncate(&option.value, 80))
                            .style(ButtonStyle::Secondary);

                        components.push(CreateActionRow::buttons(vec![button]));
                    }

                    break;
                }
            }
        }

        let embed = CreateEmbed::new()
            .title(&chat.name)
            // this may be janky, possibly rework the limit
            .description(truncate(content, 4000))
            .color(data.config().embed_color);

        CreateReply::new().embed(embed).components(components)
    }
}

impl ButtonMessage for View {
    fn edit_reply(self, ctx: ButtonContext<'_>) -> Result<EditReply<'_>> {
        let chat = ctx
            .data
            .azur_lane()
            .juustagram_chat_by_id(self.chat_id)
            .ok_or(AzurParseError::JuustagramChat)?;

        let create = self.create_with_chat(ctx.data, chat);
        Ok(create.into())
    }
}
