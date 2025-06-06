use arrayvec::ArrayVec;
use azur_lane::juustagram::*;
use utils::text::{WriteStr as _, truncate};

use super::{AzurParseError, acknowledge_unloaded};
use crate::buttons::prelude::*;
use crate::config::emoji;
use crate::fmt::discord::escape_markdown;
use crate::modules::azur::{GameData, LoadedConfig};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct View<'v> {
    chat_id: u32,
    flags: ArrayVec<u8, 20>,
    #[serde(borrow)]
    back: Option<Nav<'v>>,
}

impl<'v> View<'v> {
    pub fn new(chat_id: u32) -> Self {
        let mut flags = ArrayVec::new();
        flags.push(0u8);

        Self {
            chat_id,
            flags,
            back: None,
        }
    }

    pub fn back(mut self, back: Nav<'v>) -> Self {
        self.back = Some(back);
        self
    }

    /// Modifies the create-reply with preresolved ship data.
    fn create_with_chat<'a>(
        mut self,
        data: &'a HBotData,
        azur: LoadedConfig<'a>,
        chat: &'a Chat,
    ) -> CreateReply<'a> {
        let mut content = String::new();
        let mut components = Vec::new();

        let mut nav_row = Vec::new();
        if let Some(back) = &self.back {
            nav_row.push(
                CreateButton::new(back.to_custom_id())
                    .emoji(emoji::back())
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

        fn get_sender_name(azur: &GameData, sender_id: u32) -> &str {
            if sender_id == 0 {
                return "<You>";
            }

            azur.ship_by_id(sender_id).map_or("<unknown>", |s| &s.name)
        }

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

button_value!(for<'v> View<'v>, 16);
impl ButtonReply for View<'_> {
    async fn reply(self, ctx: ButtonContext<'_>) -> Result {
        acknowledge_unloaded(&ctx).await?;

        let azur = ctx.data.config().azur()?;
        let chat = azur
            .game_data()
            .juustagram_chat_by_id(self.chat_id)
            .ok_or(AzurParseError::JuustagramChat)?;

        let create = self.create_with_chat(ctx.data, azur, chat);
        ctx.edit(create.into()).await
    }
}
