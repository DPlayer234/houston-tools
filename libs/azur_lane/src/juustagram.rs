use serde::{Deserialize, Serialize};
use small_fixed_array::{FixedArray, FixedString};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chat {
    // "id"
    pub chat_id: u32,
    // "ship_group"
    pub group_id: u32,
    pub name: FixedString,
    pub unlock_desc: FixedString,
    // "content"
    pub entries: FixedArray<ChatEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEntry {
    // "id"
    pub entry_id: u32,
    pub content: ChatContent,
    pub flag: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<FixedArray<ChatOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatContent {
    // type = 1
    Message {
        // "ship_group"
        sender_id: u32,
        // "param"
        text: FixedString,
    },
    // type = 4
    Sticker {
        // "ship_group"
        sender_id: u32,
        // "param"
        label: FixedString,
    },
    // type = 5
    System {
        text: FixedString,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatOption {
    pub flag: u8,
    pub value: FixedString,
}
