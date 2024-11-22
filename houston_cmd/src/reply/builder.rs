use std::borrow::Cow;

use serenity::builder::{
    CreateActionRow, CreateAllowedMentions, CreateAttachment, CreateEmbed,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
    EditInteractionResponse,
};

#[derive(Debug, Default, Clone)]
pub struct CreateReply<'a> {
    content: Option<Cow<'a, str>>,
    embeds: Vec<CreateEmbed<'a>>,
    attachments: Vec<CreateAttachment<'a>>,
    components: Option<Cow<'a, [CreateActionRow<'a>]>>,
    pub(crate) ephemeral: Option<bool>,
    pub(crate) allowed_mentions: Option<CreateAllowedMentions<'a>>,
}

impl<'a> CreateReply<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the content of the message.
    pub fn content(mut self, content: impl Into<Cow<'a, str>>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Adds an embed to the message.
    ///
    /// Existing embeds are kept.
    pub fn embed(mut self, embed: CreateEmbed<'a>) -> Self {
        self.embeds.push(embed);
        self
    }

    /// Set components (buttons and select menus) for this message.
    ///
    /// Any previously set components will be overwritten.
    pub fn components(
        mut self,
        components: impl Into<Cow<'a, [CreateActionRow<'a>]>>,
    ) -> Self {
        self.components = Some(components.into());
        self
    }

    /// Add an attachment.
    pub fn attachment(mut self, attachment: CreateAttachment<'a>) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Toggles whether the message is an ephemeral response (only invoking user can see it).
    ///
    /// This only has an effect in slash commands!
    pub fn ephemeral(mut self, ephemeral: bool) -> Self {
        self.ephemeral = Some(ephemeral);
        self
    }

    /// Set the allowed mentions for the message.
    ///
    /// See [`serenity::CreateAllowedMentions`] for more information.
    pub fn allowed_mentions(
        mut self,
        allowed_mentions: CreateAllowedMentions<'a>,
    ) -> Self {
        self.allowed_mentions = Some(allowed_mentions);
        self
    }

    pub fn into_interaction_response(self) -> CreateInteractionResponseMessage<'a> {
        let Self { content, embeds, attachments, components, ephemeral, allowed_mentions } = self;

        let mut builder = CreateInteractionResponseMessage::new()
            .add_files(attachments)
            .embeds(embeds);

        if let Some(content) = content {
            builder = builder.content(content);
        }
        if let Some(components) = components {
            builder = builder.components(components);
        }
        if let Some(ephemeral) = ephemeral {
            builder = builder.ephemeral(ephemeral);
        }
        if let Some(allowed_mentions) = allowed_mentions {
            builder = builder.allowed_mentions(allowed_mentions);
        }

        builder
    }

    pub fn into_interaction_followup(self) -> CreateInteractionResponseFollowup<'a> {
        let Self { content, embeds, attachments, components, ephemeral, allowed_mentions } = self;

        let mut builder = CreateInteractionResponseFollowup::new()
            .add_files(attachments)
            .embeds(embeds);

        if let Some(content) = content {
            builder = builder.content(content);
        }
        if let Some(components) = components {
            builder = builder.components(components);
        }
        if let Some(ephemeral) = ephemeral {
            builder = builder.ephemeral(ephemeral);
        }
        if let Some(allowed_mentions) = allowed_mentions {
            builder = builder.allowed_mentions(allowed_mentions);
        }

        builder
    }

    pub fn into_interaction_edit(self) -> EditInteractionResponse<'a> {
        let Self { content, embeds, attachments, components, ephemeral: _, allowed_mentions } = self;

        let mut builder = EditInteractionResponse::new()
            .embeds(embeds);

        if let Some(content) = content {
            builder = builder.content(content);
        }
        if let Some(components) = components {
            builder = builder.components(components);
        }
        if let Some(allowed_mentions) = allowed_mentions {
            builder = builder.allowed_mentions(allowed_mentions);
        }

        for attachment in attachments {
            builder = builder.new_attachment(attachment);
        }

        builder
    }
}
