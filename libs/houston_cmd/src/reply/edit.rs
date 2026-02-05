use std::borrow::Cow;

use serde::Serialize;
use serenity::builder::{
    CreateAllowedMentions, CreateAttachment, CreateComponent, CreateEmbed, EditInteractionResponse,
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
    attachments: Option<InEditAttachments<'a>>,
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
            attachments: Some(InEditAttachments::default()),
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
        self.embeds.get_or_insert_default().push(embed);
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
    pub fn new_attachment(self, attachment: CreateAttachment<'a>) -> Self {
        self.attachment(Attachment::New(attachment))
    }

    /// Keeps an existing attachment with the given ID.
    pub fn keep_existing_attachment(self, attachment_id: AttachmentId) -> Self {
        self.attachment(Attachment::Existing(ExistingAttachment {
            id: attachment_id,
        }))
    }

    /// Removes all attachments already present.
    pub fn clear_attachments(mut self) -> Self {
        self.attachments.get_or_insert_default();
        self
    }

    fn attachment(mut self, attachment: Attachment<'a>) -> Self {
        self.attachments
            .get_or_insert_default()
            .vec
            .push(attachment);
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
            attachments,
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
            for attachment in attachments.vec {
                match attachment {
                    Attachment::New(att) => builder = builder.new_attachment(att),
                    Attachment::Existing(att) => builder = builder.keep_existing_attachment(att.id),
                }
            }
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

        let attachments = attachments.into_iter().map(Attachment::New).collect();

        Self {
            content: Some(content),
            embeds: Some(embeds),
            attachments: Some(InEditAttachments { vec: attachments }),
            components: Some(components),
            allowed_mentions,
            flags: Some(flags),
        }
    }
}

// CMBK:
// Custom support for complete interaction message edit.
// Serenity currently doesn't support a couple things when editing interaction
// responses and follow-ups, most notable keeping existing attachments.
// This may be incomplete in other ways, but is sufficient for houston-app
// purposes.

/// This type replicates logic that is performed by [`EditAttachments`].
/// However i want to avoid cloning the data here, and we can't use that
/// type directly since we need to access the internal data anyways.
#[derive(Debug, Default, Clone)]
struct InEditAttachments<'a> {
    vec: Vec<Attachment<'a>>,
}

#[derive(Debug, Clone, Serialize)]
struct ExistingAttachment {
    id: AttachmentId,
}

#[derive(Debug, Clone)]
enum Attachment<'a> {
    New(CreateAttachment<'a>),
    Existing(ExistingAttachment),
}

impl<'a> InEditAttachments<'a> {
    fn get_files(&self) -> Vec<CreateAttachment<'a>> {
        self.vec
            .iter()
            .filter_map(|e| match e {
                Attachment::New(attachment) => Some(attachment.clone()),
                _ => None,
            })
            .collect()
    }
}

impl Serialize for InEditAttachments<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq as _;

        #[derive(Debug, Clone, Serialize)]
        struct NewAttachment<'a> {
            id: u64,
            filename: &'a str,
            description: Option<&'a str>,
        }

        let mut id = 0;
        let mut seq = serializer.serialize_seq(Some(self.vec.len()))?;
        for attachment in &self.vec {
            match attachment {
                Attachment::New(new_attachment) => {
                    let attachment = NewAttachment {
                        id,
                        filename: &new_attachment.filename,
                        description: new_attachment.description.as_deref(),
                    };
                    id += 1;
                    seq.serialize_element(&attachment)?;
                },
                Attachment::Existing(existing_attachment) => {
                    seq.serialize_element(existing_attachment)?;
                },
            }
        }

        seq.end()
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
    attachments: Option<InEditAttachments<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    components: Option<Cow<'a, [CreateComponent<'a>]>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_mentions: Option<CreateAllowedMentions<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<MessageFlags>,
}

impl<'a> EditData<'a> {
    fn attachments(&self) -> Vec<CreateAttachment<'a>> {
        self.attachments
            .as_ref()
            .map_or_else(Vec::new, InEditAttachments::get_files)
    }
}

// internal workarounds for things not directly supported in serenity
impl<'a> EditReply<'a> {
    fn into_payload(self) -> EditData<'a> {
        let Self {
            content,
            embeds,
            attachments,
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
        self,
        http: &serenity::http::Http,
        interaction_id: InteractionId,
        interaction_token: &str,
    ) -> serenity::Result<()> {
        #[derive(Serialize)]
        struct Payload<'a> {
            r#type: u8,
            data: EditData<'a>,
        }

        let payload = Payload {
            r#type: 7, // UPDATE_MESSAGE
            data: self.into_payload(),
        };

        let files = payload.data.attachments();

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
        self,
        http: &serenity::http::Http,
        interaction_token: &str,
        message_id: MessageId,
    ) -> serenity::Result<Message> {
        let payload = self.into_payload();
        let files = payload.attachments();

        http.edit_followup_message(interaction_token, message_id, &payload, files)
            .await
    }
}
