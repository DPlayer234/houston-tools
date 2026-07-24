use std::borrow::Cow;
use std::mem::take;

use serde::Serialize;
use serenity::builder::{
    AttachmentData, CreateAllowedMentions, CreateAttachment, CreateComponent, CreateEmbed,
    EditAttachments, EditInteractionResponse,
};
use serenity::model::channel::{Message, MessageFlags};
use serenity::model::id::{AttachmentId, InteractionId, MessageId};

use super::CreateReply;

/// Allows building an edit, abstracting away
/// the differences between different kinds of edits.
#[derive(Debug, Default, Clone)]
#[must_use]
pub struct EditReply<'a> {
    content: Option<Cow<'a, str>>,
    embeds: Option<Vec<CreateEmbed<'a>>>,
    edit_attachments: Option<EditAttachments<'a>>,
    attachment_data: Vec<AttachmentData<'a>>,
    components: Option<Cow<'a, [CreateComponent<'a>]>>,
    allowed_mentions: Option<CreateAllowedMentions<'a>>,
    flags: Option<MessageFlags>,
}

impl<'a> EditReply<'a> {
    /// Creates a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new empty builder, which has all fields set to empty rather
    /// than absent.
    pub fn clear() -> Self {
        Self {
            content: Some(Cow::Borrowed("")),
            embeds: Some(Vec::new()),
            components: Some(Cow::Borrowed(&[])),
            edit_attachments: Some(EditAttachments::new()),
            attachment_data: Vec::new(),
            allowed_mentions: None,
            flags: None,
        }
    }

    /// Set the content of the message.
    pub fn content(mut self, content: impl Into<Cow<'a, str>>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Adds a new embed to the message.
    pub fn embed(mut self, embed: CreateEmbed<'a>) -> Self {
        let embeds = self.embeds.get_or_insert_default();
        // almost always used with just one embed
        embeds.reserve_exact(1);
        embeds.push(embed);
        self
    }

    /// Set components for this message.
    pub fn components(mut self, components: impl Into<Cow<'a, [CreateComponent<'a>]>>) -> Self {
        self.components = Some(components.into());
        self
    }

    /// Set components for this message.
    pub fn components_v2(mut self, components: impl Into<Cow<'a, [CreateComponent<'a>]>>) -> Self {
        self.flags
            .get_or_insert_default()
            .insert(MessageFlags::IS_COMPONENTS_V2);
        self.components(components)
    }

    /// Add a new attachment.
    pub fn new_attachment(mut self, attachment: CreateAttachment<'a>) -> Self {
        // don't like this clone, but we can't get the data otherwise, should also be
        // cheap enough since it at worst clones the description redundantly
        self.attachment_data.push(attachment.clone().into());
        self.edit_attachments = Some(
            self.edit_attachments
                .take()
                .unwrap_or_default()
                .add(attachment),
        );
        self
    }

    /// Keeps an existing attachment with the given ID.
    pub fn keep_existing_attachment(mut self, attachment_id: AttachmentId) -> Self {
        self.edit_attachments = Some(
            self.edit_attachments
                .take()
                .unwrap_or_default()
                .keep(attachment_id),
        );
        self
    }

    /// Removes all attachments already present.
    pub fn clear_attachments(mut self) -> Self {
        self.edit_attachments.get_or_insert_default();
        self
    }

    /// Set the allowed mentions for the message.
    pub fn allowed_mentions(mut self, allowed_mentions: CreateAllowedMentions<'a>) -> Self {
        self.allowed_mentions = Some(allowed_mentions);
        self
    }

    /// Creates an interaction edit from the builder.
    pub fn into_interaction_edit(self) -> EditInteractionResponse<'a> {
        let Self {
            content,
            embeds,
            edit_attachments: attachments,
            attachment_data: _,
            components,
            allowed_mentions,
            flags,
        } = self;

        let mut builder = EditInteractionResponse::new();

        if let Some(content) = content {
            builder = builder.content(content);
        }
        if let Some(embeds) = embeds {
            builder = builder.embeds(embeds);
        }
        if let Some(components) = components {
            builder = builder.components(components);
        }
        if let Some(allowed_mentions) = allowed_mentions {
            builder = builder.allowed_mentions(allowed_mentions);
        }
        if let Some(flags) = flags {
            builder = builder.flags(flags);
        }
        if let Some(attachments) = attachments {
            builder = builder.attachments(attachments);
        }

        builder
    }
}

impl<'a> From<CreateReply<'a>> for EditReply<'a> {
    /// Creates an edit that will put the message into the same state as the
    /// message this would create.
    ///
    /// This means, that unless specified as non-empty in the source value,
    /// the resulting will clear content, embeds, components, and attachments.
    fn from(value: CreateReply<'a>) -> Self {
        let CreateReply {
            content,
            embeds,
            attachments,
            components,
            allowed_mentions,
            flags,
        } = value;

        let mut edit_attachments = Some(EditAttachments::new());
        let attachment_data = attachments
            .into_iter()
            .inspect(|a| {
                edit_attachments = Some(edit_attachments.take().unwrap_or_default().add(a.clone()));
            })
            .map(AttachmentData::from)
            .collect();

        Self {
            content: Some(content),
            embeds: Some(embeds),
            edit_attachments,
            attachment_data,
            components: Some(components),
            allowed_mentions,
            flags: Some(flags),
        }
    }
}

/// Essentially just [`EditReply`] but with serialization support.
#[derive(Serialize)]
struct EditData<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<Cow<'a, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embeds: Option<Vec<CreateEmbed<'a>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<EditAttachments<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    components: Option<Cow<'a, [CreateComponent<'a>]>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_mentions: Option<CreateAllowedMentions<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<MessageFlags>,
}

// internal workarounds for things not directly supported in serenity
impl<'a> EditReply<'a> {
    fn into_payload(self) -> EditData<'a> {
        let Self {
            content,
            embeds,
            edit_attachments: attachments,
            attachment_data: _,
            components,
            allowed_mentions,
            flags,
        } = self;

        EditData {
            content,
            embeds,
            attachments,
            components,
            allowed_mentions,
            flags,
        }
    }

    /// Invokes [`create_interaction_response`] with the correct information for
    /// an edit. This works around [`CreateInteractionResponse::UpdateMessage`]
    /// not supporting keeping existing attachments.
    ///
    /// Hidden because I don't want this in the public API but I do need it in
    /// `houston_btn`.
    ///
    /// [`create_interaction_response`]: serenity::http::Http::create_interaction_response
    #[doc(hidden)]
    pub async fn execute_as_response(
        mut self,
        http: &serenity::http::Http,
        interaction_id: InteractionId,
        interaction_token: &str,
    ) -> serenity::Result<()> {
        #[derive(Serialize)]
        struct Payload<'a> {
            r#type: u8,
            data: EditData<'a>,
        }

        let files = take(&mut self.attachment_data);
        let payload = Payload {
            r#type: 7, // UPDATE_MESSAGE
            data: self.into_payload(),
        };

        http.create_interaction_response(interaction_id, interaction_token, &payload, files)
            .await
    }

    /// Invokes [`edit_followup_message`] with the correct information for an
    /// edit. This works around [`CreateInteractionResponseFollowup`] being used
    /// for edits but not supporting keeping existing attachments.
    ///
    /// Hidden because I don't want this in the public API but I do need it in
    /// `houston_btn`.
    ///
    /// [`edit_followup_message`]: serenity::http::Http::edit_followup_message
    #[doc(hidden)]
    pub async fn execute_as_followup_edit(
        mut self,
        http: &serenity::http::Http,
        interaction_token: &str,
        message_id: MessageId,
    ) -> serenity::Result<Message> {
        let files = take(&mut self.attachment_data);
        let payload = self.into_payload();

        http.edit_followup_message(interaction_token, message_id, &payload, files)
            .await
    }
}
