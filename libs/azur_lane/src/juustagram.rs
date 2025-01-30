use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chat {
    // "id"
    pub chat_id: u32,
    // "ship_group"
    pub group_id: u32,
    pub name: String,
    pub unlock_desc: String,
    // "content"
    pub entries: Vec<ChatEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEntry {
    // "id"
    pub entry_id: u32,
    pub content: ChatContent,
    pub flag: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<ChatOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatContent {
    // type = 1
    Message {
        // "ship_group"
        sender_id: u32,
        // "param"
        text: String,
    },
    // type = 4
    Sticker {
        // "ship_group"
        sender_id: u32,
        // "param"
        label: String,
    },
    // type = 5
    System {
        text: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatOption {
    pub flag: u8,
    pub value: String,
}
