use crate::prelude::*;
use crate::fmt::discord::get_unique_username;

/// Creates a copyable, quotable version of the message.
#[poise::command(context_menu_command = "Get as Quote")]
pub async fn quote(
    ctx: HContext<'_>,
    message: Message,
) -> HResult {
    let content = format!(
        "```\n{}\n```",
        QuoteContent::new(&ctx, &message)
    );

    let embed = CreateEmbed::new()
        .description(content)
        .color(DEFAULT_EMBED_COLOR);

    ctx.send(ctx.create_ephemeral_reply().embed(embed)).await?;
    Ok(())
}

struct QuoteContent<'a> {
    ctx: &'a HContext<'a>,
    message: &'a Message
}

impl<'a> QuoteContent<'a> {
    fn new(ctx: &'a HContext<'a>, message: &'a Message) -> Self {
        Self { ctx, message }
    }
}

impl std::fmt::Display for QuoteContent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for line in self.message.content.lines() {
            f.write_str("> ")?;
            f.write_str(line)?;
            f.write_str("\n")?;
        }

        write!(
            f,
            "-# \\- {} @ <t:{}> {}",
            get_unique_username(&self.message.author),
            self.message.timestamp.unix_timestamp(),
            self.message.id.link(self.ctx.channel_id(), self.ctx.guild_id()),
        )
    }
}
