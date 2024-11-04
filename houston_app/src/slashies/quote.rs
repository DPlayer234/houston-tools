use std::fmt;

use utils::time::*;

use crate::fmt::discord::get_unique_username;
use crate::prelude::*;

/// Creates a copyable, quotable version of the message.
#[poise::command(context_menu_command = "Get as Quote")]
pub async fn quote(
    ctx: HContext<'_>,
    mut message: Message,
) -> HResult {
    // seemingly not always correctly set for messages received in interactions
    message.channel_id = ctx.channel_id();
    message.guild_id = ctx.guild_id();

    let content = format!(
        "-# Quote: {t:x}\n```\n{t}\n```",
        t = QuoteTarget::new(&message)
    );

    let embed = CreateEmbed::new()
        .description(content)
        .color(DEFAULT_EMBED_COLOR);

    ctx.send(ctx.create_ephemeral_reply().embed(embed)).await?;
    Ok(())
}

struct QuoteTarget<'a> {
    message: &'a Message,
}

impl<'a> QuoteTarget<'a> {
    fn new(message: &'a Message) -> Self {
        Self { message }
    }
}

impl fmt::LowerHex for QuoteTarget<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let channel_id = self.message.channel_id;
        let message_id = self.message.id;

        if let Some(guild_id) = self.message.guild_id {
            write!(f, "https://discord.com/channels/{guild_id}/{channel_id}/{message_id}")
        } else {
            write!(f, "https://discord.com/channels/@me/{channel_id}/{message_id}")
        }
    }
}

impl fmt::Display for QuoteTarget<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in self.message.content.lines() {
            f.write_str("> ")?;
            f.write_str(line)?;
            f.write_str("\n")?;
        }

        write!(
            f,
            "-# \\- {name} @ {time} {link:x}",
            name = get_unique_username(&self.message.author),
            time = self.message.timestamp.short_date_time(),
            link = *self,
        )
    }
}


