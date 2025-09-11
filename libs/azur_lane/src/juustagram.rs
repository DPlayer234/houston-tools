//! Data model for Juustagram chats.

use serde::{Deserialize, Serialize};
use small_fixed_array::{FixedArray, FixedString};

/// A Juustagram chat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chat {
    /// The game ID for the chat.
    pub chat_id: u32,
    /// The `ship_group` that this chat is associated with, or 0.
    pub group_id: u32,
    /// The name of the chat.
    pub name: FixedString,
    /// The description for the unlock condition.
    ///
    /// As far as I can tell, this is unused in the game.
    pub unlock_desc: FixedString,
    /// The `content` entries for the chat, i.e. the actual messages.
    pub entries: FixedArray<ChatEntry>,
}

/// A Juustagram chat entry, i.e. an actual message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEntry {
    /// The game ID for the entry.
    pub entry_id: u32,
    /// The content of the message.
    pub content: ChatContent,
    /// The choice flag for this message.
    ///
    /// This means that a message is only shown when an option has been chosen
    /// that had this flag. 0 indicates it is always shown.
    pub flag: u8,
    /// Option choices, if there are any to select after this message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<FixedArray<ChatOption>>,
}

/// The chat message content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatContent {
    /// Type 1: A plain text message.
    Message {
        /// The ship group ID of the sender ship.
        sender_id: u32,
        /// The text sent.
        text: FixedString,
    },
    /// Type 4: A sticker.
    Sticker {
        /// The ship group ID of the sender ship.
        sender_id: u32,
        /// The sticker's label.
        label: FixedString,
    },
    /// Type 5: A "system" message, i.e. that someone is typing.
    System {
        /// The text content.
        text: FixedString,
    },
}

/// A chat option choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatOption {
    /// The flag set by this option.
    ///
    /// This enables messages with the same flag when chosen.
    pub flag: u8,
    /// The text content for this option.
    pub value: FixedString,
}
