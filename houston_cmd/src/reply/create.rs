use std::borrow::Cow;

use serenity::builder::*;

/// Allows building a reply to an interaction, abstracting away
/// the differences between initial responses, follow-ups, and edits.
#[derive(Debug, Default, Clone)]
pub struct CreateReply<'a> {
    pub(crate) content: Cow<'a, str>,
    pub(crate) embeds: Vec<CreateEmbed<'a>>,
    pub(crate) attachments: Vec<CreateAttachment<'a>>,
    pub(crate) components: Cow<'a, [CreateActionRow<'a>]>,
    pub(crate) ephemeral: Option<bool>,
    pub(crate) allowed_mentions: Option<CreateAllowedMentions<'a>>,
}

impl<'a> CreateReply<'a> {
    /// Creates a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the content of the message.
    pub fn content(mut self, content: impl Into<Cow<'a, str>>) -> Self {
        self.content = content.into();
        self
    }

    /// Adds a new embed to the message.
    pub fn embed(mut self, embed: CreateEmbed<'a>) -> Self {
        self.embeds.push(embed);
        self
    }

    /// Set components for this message.
    pub fn components(
        mut self,
        components: impl Into<Cow<'a, [CreateActionRow<'a>]>>,
    ) -> Self {
        self.components = components.into();
        self
    }

    /// Add an attachment.
    pub fn attachment(mut self, attachment: CreateAttachment<'a>) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Sets whether the message is ephemeral.
    ///
    /// This has no effect on edits.
    pub fn ephemeral(mut self, ephemeral: bool) -> Self {
        self.ephemeral = Some(ephemeral);
        self
    }

    /// Set the allowed mentions for the message.
    pub fn allowed_mentions(mut self, allowed_mentions: CreateAllowedMentions<'a>) -> Self {
        self.allowed_mentions = Some(allowed_mentions);
        self
    }

    /// Creates an interaction response message from the builder.
    pub fn into_interaction_response(self) -> CreateInteractionResponseMessage<'a> {
        let Self { content, embeds, attachments, components, ephemeral, allowed_mentions } = self;

        let mut builder = CreateInteractionResponseMessage::new()
            .content(content)
            .embeds(embeds)
            .components(components)
            .add_files(attachments);

        if let Some(ephemeral) = ephemeral {
            builder = builder.ephemeral(ephemeral);
        }
        if let Some(allowed_mentions) = allowed_mentions {
            builder = builder.allowed_mentions(allowed_mentions);
        }

        builder
    }

    /// Creates an interaction followup from the builder.
    pub fn into_interaction_followup(self) -> CreateInteractionResponseFollowup<'a> {
        let Self { content, embeds, attachments, components, ephemeral, allowed_mentions } = self;

        let mut builder = CreateInteractionResponseFollowup::new()
            .content(content)
            .embeds(embeds)
            .components(components)
            .add_files(attachments);

        if let Some(ephemeral) = ephemeral {
            builder = builder.ephemeral(ephemeral);
        }
        if let Some(allowed_mentions) = allowed_mentions {
            builder = builder.allowed_mentions(allowed_mentions);
        }

        builder
    }

    /// Creates an interaction edit from the builder.
    pub fn into_interaction_edit(self) -> EditInteractionResponse<'a> {
        let Self { content, embeds, attachments, components, ephemeral: _, allowed_mentions } = self;

        let mut builder = EditInteractionResponse::new()
            .content(content)
            .embeds(embeds)
            .components(components)
            .clear_attachments();

        if let Some(allowed_mentions) = allowed_mentions {
            builder = builder.allowed_mentions(allowed_mentions);
        }

        for attachment in attachments {
            builder = builder.new_attachment(attachment);
        }

        builder
    }
}