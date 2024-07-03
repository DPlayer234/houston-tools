use std::io::Write;

use crate::prelude::*;

/// Uploads a file to a temporary directory.
#[poise::command(slash_command)]
pub async fn upload(
    ctx: HContext<'_>,
    #[description = "The file to upload."]
	attachment: Attachment
) -> HResult {
    utils::define_simple_error!(NotEnabled: "upload is not enabled");

    let config = ctx.data().config();
    if !config.trusted_users.contains(&ctx.author().id.get()) {
        let embed = CreateEmbed::new()
            .color(ERROR_EMBED_COLOR)
            .description("You aren't listed in `trusted_users`.");

        ctx.send(ctx.create_ephemeral_reply().embed(embed)).await?;
        return Ok(());
    }

    let mut filename = config.upload_dir.clone().ok_or(NotEnabled)?;
    std::fs::create_dir_all(&filename)?;

    filename.push(sanitize(&attachment.filename));

    let embed = CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .description(format!("Saving to {filename:?}."));

    let reply = ctx.send(ctx.create_ephemeral_reply().embed(embed)).await?;

    {
        let mut file = std::fs::OpenOptions::new().create_new(true).write(true).open(&filename)?;
        let data = attachment.download().await?;
        file.write_all(&data)?;
    }

    let embed = CreateEmbed::new()
        .color(DEFAULT_EMBED_COLOR)
        .description(format!("Saved as {filename:?}!"));

    reply.edit(ctx, ctx.create_ephemeral_reply().embed(embed)).await?;
	Ok(())
}

fn sanitize(name: &str) -> String {
    name.replace(|c: char| c != '.' && !c.is_alphanumeric(), "_")
}
