use crate::prelude::*;
use crate::fmt::discord::get_unique_username;

/// Creates a copyable, quotable version of the message.
#[poise::command(context_menu_command = "Get as Quote")]
pub async fn quote(
    ctx: HContext<'_>,
    message: Message,
) -> HResult {
    let content = format!(
        "-# Quote: {t:x}\n```\n{t}\n```",
        t = QuoteTarget::new(&ctx, &message)
    );

    let embed = CreateEmbed::new()
        .description(content)
        .color(DEFAULT_EMBED_COLOR);

    ctx.send(ctx.create_ephemeral_reply().embed(embed)).await?;
    Ok(())
}

struct QuoteTarget<'a> {
    ctx: &'a HContext<'a>,
    message: &'a Message
}

impl<'a> QuoteTarget<'a> {
    fn new(ctx: &'a HContext<'a>, message: &'a Message) -> Self {
        Self { ctx, message }
    }
}

impl std::fmt::LowerHex for QuoteTarget<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let channel_id = self.ctx.channel_id();
        let message_id = self.message.id;

        if let Some(guild_id) = self.ctx.guild_id() {
            write!(f, "https://discord.com/channels/{guild_id}/{channel_id}/{message_id}")
        } else {
            write!(f, "https://discord.com/channels/@me/{channel_id}/{message_id}")
        }
    }
}

impl std::fmt::Display for QuoteTarget<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for line in self.message.content.lines() {
            f.write_str("> ")?;
            f.write_str(line)?;
            f.write_str("\n")?;
        }

        write!(
            f,
            "-# \\- {} @ <t:{}> {:x}",
            get_unique_username(&self.message.author),
            self.message.timestamp.unix_timestamp(),
            *self,
        )
    }
}


