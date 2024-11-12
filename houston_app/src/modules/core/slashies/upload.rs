use std::fmt;

use crate::prelude::*;

/// Uploads a file to an ephemeral message. Allows sharing if you are logged into multiple devices.
#[poise::command(slash_command)]
pub async fn upload(
    ctx: HContext<'_>,
    #[description = "The file to upload."]
    attachment: Attachment
) -> HResult {
    let description = format!(
        "**{}**\n> {}",
        attachment.filename,
        StorageSize(attachment.size)
    );

    let mut embed = CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .description(description);

    if attachment.dimensions().is_some() {
        embed = embed.thumbnail(attachment.proxy_url.as_str());
    }

    let buttons = [
        CreateButton::new_link(attachment.url)
            .label("Download")
    ];

    let components = [
        CreateActionRow::buttons(&buttons)
    ];

    let reply = ctx.create_ephemeral_reply()
        .embed(embed)
        .components(&components);

    ctx.send(reply).await?;
    Ok(())
}

struct StorageSize(u32);

impl fmt::Display for StorageSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const FACTOR: u32 = 1024;
        const KB: u32 = FACTOR;
        const MB: u32 = KB * FACTOR;
        const KB_LIMIT: u32 = MB - 1;

        match self.0 {
            s @ ..=KB_LIMIT => write!(f, "{:.1} KB", f64::from(s) / f64::from(KB)),
            s => write!(f, "{:.1} MB", f64::from(s) / f64::from(MB)),
        }
    }
}
