use std::collections::VecDeque;

use serenity::small_fixed_array::{FixedArray, FixedString};

use crate::prelude::*;
use crate::slashies::args::SlashUser;

#[derive(Debug, Default)]
pub struct GuildState {
    /// The received messages, including ones already deleted.
    ///
    /// These may be slightly ouf of order.
    pub messages: VecDeque<SnipedMessage>,
}

#[derive(Debug, Clone)]
pub struct SnipedMessage {
    pub id: MessageId,
    pub channel_id: GenericChannelId,
    pub author: SnipedAuthor,
    pub content: FixedString<u16>,
    pub attachments: FixedArray<SnipedAttachment, u8>,
    pub deleted: bool,
}

#[derive(Debug, Clone)]
pub struct SnipedAuthor {
    pub display_name: FixedString<u8>,
    pub avatar_url: FixedString<u8>,
}

#[derive(Debug, Clone)]
pub struct SnipedAttachment {
    pub filename: FixedString<u8>,
    pub url: FixedString,
}

fn capture_attachments(attachments: &[Attachment]) -> FixedArray<SnipedAttachment, u8> {
    let attachments = attachments
        .iter()
        .map(|a| SnipedAttachment {
            filename: FixedString::from_str_trunc(a.filename.as_str()),
            url: a.url.clone(),
        })
        .collect();

    FixedArray::from_vec_trunc(attachments)
}

impl SnipedMessage {
    pub fn new(msg: &Message) -> Self {
        let author = SlashUser::from_message(msg);
        let author = SnipedAuthor {
            display_name: FixedString::from_str_trunc(author.display_name()),
            avatar_url: FixedString::from_string_trunc(author.face()),
        };

        Self {
            id: msg.id,
            channel_id: msg.channel_id,
            author,
            content: msg.content.clone(),
            attachments: capture_attachments(&msg.attachments),
            deleted: false,
        }
    }

    pub fn update(&mut self, msg: &Message) {
        self.content.clone_from(&msg.content);
        self.attachments = capture_attachments(&msg.attachments);
    }
}

impl GuildState {
    pub fn get_message_mut(&mut self, message_id: MessageId) -> Option<&mut SnipedMessage> {
        self.messages.iter_mut().find(move |m| m.id == message_id)
    }

    pub fn take_last<F>(&mut self, mut f: F) -> Option<SnipedMessage>
    where
        F: FnMut(&SnipedMessage) -> bool,
    {
        // find the last index of a message matching the predicate
        let mut iter = self.messages.iter().enumerate();
        let (index, _) = iter.rfind(move |(_, m)| f(m))?;
        self.messages.remove(index)
    }
}
