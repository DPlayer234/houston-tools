use std::fmt;

use crate::fmt::discord::{MessageLink, TimeMentionable, get_unique_username};
use crate::slashies::prelude::*;

/// Creates a copyable, quotable version of the message.
#[context_command(
    message,
    name = "Get as Quote",
    contexts = "Guild | BotDm | PrivateChannel",
    integration_types = "Guild | User"
)]
pub async fn quote(ctx: Context<'_>, message: &Message) -> Result {
    // seemingly not always correctly set for messages received in interactions
    let content = format!(
        "-# Quote: {t:x}\n```\n{t}\n```",
        t = QuoteTarget::new(message, ctx.channel_id(), ctx.guild_id())
    );

    let embed = CreateEmbed::new()
        .description(content)
        .color(ctx.data_ref().config().embed_color);

    ctx.send(create_reply(Ephemeral).embed(embed)).await?;
    Ok(())
}

struct QuoteTarget<'a> {
    message: &'a Message,
    channel_id: ChannelId,
    guild_id: Option<GuildId>,
}

impl<'a> QuoteTarget<'a> {
    fn new(message: &'a Message, channel_id: ChannelId, guild_id: Option<GuildId>) -> Self {
        Self {
            message,
            channel_id,
            guild_id,
        }
    }
}

impl fmt::LowerHex for QuoteTarget<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let l = MessageLink::new(self.guild_id, self.channel_id, self.message.id);
        fmt::Display::fmt(&l, f)
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
