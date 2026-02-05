use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::id::MessageId;

use super::EditReply;

/// Represents a handle to a sent interaction response or follow-up,
/// allowing edits or deletion.
#[derive(Debug, Clone, Copy)]
pub struct ReplyHandle<'a> {
    http: &'a Http,
    token: &'a str,
    target: Option<MessageId>,
}

impl<'a> ReplyHandle<'a> {
    /// Creates a new reply handle to an arbitrary interaction response or
    /// follow-up.
    ///
    /// Hidden because I don't want this in the public API but I do need it in
    /// `houston_btn`.
    #[doc(hidden)]
    pub fn new(http: &'a Http, token: &'a str, target: Option<MessageId>) -> Self {
        Self {
            http,
            token,
            target,
        }
    }

    /// Delete the message.
    #[expect(clippy::missing_errors_doc)]
    pub async fn delete(self) -> serenity::Result<()> {
        match self.target {
            None => {
                self.http
                    .delete_original_interaction_response(self.token)
                    .await?
            },
            Some(message_id) => {
                self.http
                    .delete_followup_message(self.token, message_id)
                    .await?;
            },
        }

        Ok(())
    }

    /// Edit the message.
    ///
    /// You cannot edit whether a message is ephemeral.
    #[expect(clippy::missing_errors_doc)]
    pub async fn edit(self, reply: EditReply<'_>) -> serenity::Result<Message> {
        match self.target {
            None => {
                reply
                    .into_interaction_edit()
                    .execute(self.http, self.token)
                    .await
            },
            Some(message_id) => {
                reply
                    .execute_as_followup_edit(self.http, self.token, message_id)
                    .await
            },
        }
    }
}
